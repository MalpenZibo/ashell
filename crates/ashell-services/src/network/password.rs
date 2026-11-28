//! Wi-Fi passphrases NetworkManager or iwd wait for, until the user answers.

use super::Redacted;
use futures::{Stream, channel::oneshot, stream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::watch;
use zbus::OwnedObjectPath;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PasswordRequestId(u64);

/// A daemon waiting for the passphrase of `ssid`, a WPA, WPA2 or WPA3
/// Personal Wi-Fi network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordRequest {
    pub id: PasswordRequestId,
    pub ssid: String,
}

/// The daemon waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Asker {
    /// NetworkManager, for the `setting` of the profile at `connection`.
    NetworkManager {
        connection: OwnedObjectPath,
        setting: String,
    },
    Iwd,
}

impl Asker {
    pub(super) fn is_network_manager(&self) -> bool {
        matches!(self, Asker::NetworkManager { .. })
    }

    pub(super) fn is_iwd(&self) -> bool {
        matches!(self, Asker::Iwd)
    }
}

/// How a password request ends.
#[derive(Debug)]
pub enum PasswordReply {
    Password(Redacted<String>),
    /// The user dismissed it: NetworkManager asks no other agent.
    Cancel,
    /// Nobody can answer it here: NetworkManager asks the other agents. iwd
    /// has a single agent, so for iwd it is the same as `Cancel`. Requests a
    /// daemon takes back end this way too.
    Decline,
}

#[derive(Debug)]
struct Pending {
    request: PasswordRequest,
    asker: Asker,
    /// Dropped unanswered when the request is withdrawn.
    reply: oneshot::Sender<PasswordReply>,
}

#[derive(Debug, Default)]
struct State {
    next_id: u64,
    /// Oldest first.
    pending: Vec<Pending>,
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    /// The pending requests, for every `subscribe()` stream.
    list: watch::Sender<Vec<PasswordRequest>>,
}

/// The pending requests of a handle, shared by its agents and streams.
#[derive(Debug, Clone)]
pub(super) struct PasswordRequests(Arc<Shared>);

impl Default for PasswordRequests {
    fn default() -> Self {
        Self(Arc::new(Shared {
            state: Mutex::default(),
            list: watch::Sender::new(Vec::new()),
        }))
    }
}

impl PasswordRequests {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn publish(&self, state: &State) {
        self.0.list.send_replace(
            state
                .pending
                .iter()
                .map(|pending| pending.request.clone())
                .collect(),
        );
    }

    /// Asks the user for the passphrase of `ssid`, and waits for the answer.
    /// Withdrawn at once when no stream shows requests.
    pub(super) async fn ask(&self, ssid: String, asker: Asker) -> PasswordReply {
        let (reply, answer) = oneshot::channel();
        let id = {
            let mut state = self.lock();
            if self.0.list.receiver_count() == 0 {
                return PasswordReply::Decline;
            }
            state.next_id += 1;
            let id = PasswordRequestId(state.next_id);
            state.pending.push(Pending {
                request: PasswordRequest { id, ssid },
                asker,
                reply,
            });
            self.publish(&state);
            id
        };

        // The D-Bus call can go away before an answer, taking this future with it
        let _forget = Forget { requests: self, id };
        answer.await.unwrap_or(PasswordReply::Decline)
    }

    /// Ends the request `id` with `reply`, if still pending.
    pub(super) fn reply(&self, id: PasswordRequestId, reply: PasswordReply) {
        let pending = {
            let mut state = self.lock();
            let Some(index) = state.pending.iter().position(|p| p.request.id == id) else {
                return;
            };
            let pending = state.pending.remove(index);
            self.publish(&state);
            pending
        };
        // The asker is gone when its daemon dropped the call
        let _ = pending.reply.send(reply);
    }

    /// Withdraws the pending requests of the askers `which` matches.
    pub(super) fn withdraw(&self, which: impl Fn(&Asker) -> bool) {
        self.remove(|pending| which(&pending.asker));
    }

    /// Drops the pending requests `which` matches, unanswered.
    fn remove(&self, which: impl Fn(&Pending) -> bool) {
        let mut state = self.lock();
        let before = state.pending.len();
        state.pending.retain(|pending| !which(pending));
        if state.pending.len() != before {
            self.publish(&state);
        }
    }

    /// The pending requests: the current ones, then after every change. The
    /// requests are withdrawn once no stream is left.
    pub(super) fn subscribe(&self) -> impl Stream<Item = Vec<PasswordRequest>> + Send + 'static {
        let mut list = self.0.list.subscribe();
        list.mark_changed();
        let listener = Listener {
            requests: self.clone(),
            list: Some(list),
        };

        stream::unfold(listener, |mut listener| async move {
            let list = listener.list.as_mut()?;
            list.changed().await.ok()?;
            let requests = list.borrow_and_update().clone();
            Some((requests, listener))
        })
    }
}

/// Removes a request whose asker stopped waiting.
struct Forget<'a> {
    requests: &'a PasswordRequests,
    id: PasswordRequestId,
}

impl Drop for Forget<'_> {
    fn drop(&mut self) {
        self.requests
            .remove(|pending| pending.request.id == self.id);
    }
}

struct Listener {
    requests: PasswordRequests,
    list: Option<watch::Receiver<Vec<PasswordRequest>>>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.list.take();
        if self.requests.0.list.receiver_count() == 0 {
            self.requests.withdraw(|_| true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{FutureExt, StreamExt};
    use std::pin::pin;

    fn connection(path: &str) -> Asker {
        Asker::NetworkManager {
            connection: OwnedObjectPath::try_from(path).unwrap(),
            setting: "802-11-wireless-security".to_string(),
        }
    }

    #[test]
    fn nobody_listening_withdraws_at_once() {
        let requests = PasswordRequests::default();

        let answer = requests.ask("Home".to_string(), Asker::Iwd).now_or_never();

        assert!(matches!(answer, Some(PasswordReply::Decline)));
    }

    #[test]
    fn an_answer_reaches_the_asker_and_leaves_the_list() {
        let requests = PasswordRequests::default();
        let mut list = pin!(requests.subscribe());
        assert_eq!(list.next().now_or_never(), Some(Some(Vec::new())));

        let mut ask = pin!(requests.ask("Home".to_string(), Asker::Iwd));
        assert!(ask.as_mut().now_or_never().is_none());

        let pending = list.next().now_or_never().flatten().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].ssid, "Home");

        let password = Redacted::new("hunter22".to_string());
        requests.reply(pending[0].id, PasswordReply::Password(password));
        requests.reply(pending[0].id, PasswordReply::Cancel);

        match ask.now_or_never() {
            Some(PasswordReply::Password(password)) => {
                assert_eq!(password.into_inner(), "hunter22")
            }
            other => panic!("unexpected answer: {other:?}"),
        }
        assert_eq!(list.next().now_or_never(), Some(Some(Vec::new())));
    }

    #[test]
    fn withdrawing_ends_only_the_matching_requests() {
        let requests = PasswordRequests::default();
        let mut list = pin!(requests.subscribe());
        let mut first = pin!(requests.ask("A".to_string(), connection("/c/1")));
        let mut second = pin!(requests.ask("B".to_string(), Asker::Iwd));
        assert!(first.as_mut().now_or_never().is_none());
        assert!(second.as_mut().now_or_never().is_none());

        requests.withdraw(Asker::is_network_manager);

        assert!(matches!(first.now_or_never(), Some(PasswordReply::Decline)));
        assert!(second.as_mut().now_or_never().is_none());
        let pending = list.next().now_or_never().flatten().unwrap();
        let ssids: Vec<_> = pending.iter().map(|r| r.ssid.as_str()).collect();
        assert_eq!(ssids, ["B"]);
    }

    #[test]
    fn the_last_stream_gone_withdraws_everything() {
        let requests = PasswordRequests::default();
        let list = requests.subscribe();
        let mut ask = pin!(requests.ask("Home".to_string(), Asker::Iwd));
        assert!(ask.as_mut().now_or_never().is_none());

        drop(list);

        assert!(matches!(ask.now_or_never(), Some(PasswordReply::Decline)));
    }

    #[test]
    fn an_asker_gone_leaves_the_list() {
        let requests = PasswordRequests::default();
        let mut list = pin!(requests.subscribe());
        let _ = list.next().now_or_never();

        let mut ask = Box::pin(requests.ask("Home".to_string(), Asker::Iwd));
        assert!(ask.as_mut().now_or_never().is_none());
        assert_eq!(
            list.next().now_or_never().flatten().map(|l| l.len()),
            Some(1)
        );

        drop(ask);

        assert_eq!(list.next().now_or_never(), Some(Some(Vec::new())));
    }
}

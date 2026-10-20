//! IPC via Unix domain socket.
//!
//! The daemon listens on `$XDG_RUNTIME_DIR/ashell.sock`, or on
//! `$TMPDIR/ashell-<uid>/ashell.sock` when the runtime dir is unusable.
//! The same binary acts as a client via `ashell msg <command>`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use crate::config;
use crate::xdg;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use iced::Subscription;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender as ReplySender, channel};
use tokio::task::JoinSet;
use tokio::time::timeout;

use crate::services::notifications::{Notification, Urgency};
use iced::futures::{SinkExt, channel::mpsc::Sender};

/// Maximum bytes to read from a client connection.
const MAX_REQUEST_LEN: u64 = 4096;
const MAX_CONNECTIONS: usize = 32;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// IPC command that can be sent to the daemon.
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum IpcCommand {
    /// Toggle bar visibility
    ToggleVisibility,
    VolumeUp {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    VolumeDown {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    VolumeToggleMute {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    MicrophoneUp {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    MicrophoneDown {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    MicrophoneToggleMute {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    BrightnessUp {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    BrightnessDown {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    ToggleAirplaneMode {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    ToggleIdleInhibitor {
        #[arg(long)]
        #[serde(default)]
        no_osd: bool,
    },
    LoadConfig {
        #[arg(value_parser = config::resolve_config_arg)]
        file: PathBuf,
    },
    /// Close all notifications, or a single notification by ID
    NotificationsClear {
        #[arg(long)]
        id: Option<u32>,
    },
    /// Invoke an action and close its notification
    NotificationsInvoke {
        #[arg(long)]
        id: Option<u32>,
        #[arg(default_value = "default")]
        #[serde(default = "default_action")]
        action: String,
    },
    /// Print retained notifications as a JSON array, newest first
    NotificationsList,
}

fn default_action() -> String {
    "default".into()
}

impl IpcCommand {
    pub fn no_osd(&self) -> bool {
        match self {
            IpcCommand::ToggleVisibility => false,
            IpcCommand::VolumeUp { no_osd }
            | IpcCommand::VolumeDown { no_osd }
            | IpcCommand::VolumeToggleMute { no_osd }
            | IpcCommand::MicrophoneUp { no_osd }
            | IpcCommand::MicrophoneDown { no_osd }
            | IpcCommand::MicrophoneToggleMute { no_osd }
            | IpcCommand::BrightnessUp { no_osd }
            | IpcCommand::BrightnessDown { no_osd }
            | IpcCommand::ToggleAirplaneMode { no_osd }
            | IpcCommand::ToggleIdleInhibitor { no_osd } => *no_osd,
            IpcCommand::LoadConfig { .. }
            | IpcCommand::NotificationsClear { .. }
            | IpcCommand::NotificationsInvoke { .. }
            | IpcCommand::NotificationsList => false,
        }
    }
}

/// A newline-delimited JSON reply to a socket request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notifications: Option<Vec<NotificationInfo>>,
    },
    Error {
        message: String,
    },
}

impl IpcResponse {
    pub fn ok() -> Self {
        Self::Ok {
            notifications: None,
        }
    }

    pub fn error(message: impl ToString) -> Self {
        Self::Error {
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInfo {
    id: u32,
    app_name: String,
    summary: String,
    body: String,
    urgency: String,
    actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationAction {
    key: String,
    label: String,
}

impl From<&Notification> for NotificationInfo {
    fn from(notification: &Notification) -> Self {
        Self {
            id: notification.id,
            app_name: notification.app_name.clone(),
            summary: notification.summary.clone(),
            body: notification.body.clone(),
            urgency: match notification.urgency {
                Urgency::Low => "low",
                Urgency::Normal => "normal",
                Urgency::Critical => "critical",
            }
            .into(),
            actions: notification
                .action_pairs()
                .map(|(key, label)| NotificationAction {
                    key: key.to_owned(),
                    label: label.to_owned(),
                })
                .collect(),
        }
    }
}

/// A cloneable reply handle so requests can travel through iced messages/tasks.
#[derive(Debug, Clone)]
pub struct IpcReply(ReplySender<IpcResponse>);

impl IpcReply {
    pub fn channel() -> (Self, Receiver<IpcResponse>) {
        let (sender, receiver) = channel(1);
        (Self(sender), receiver)
    }

    pub fn send(&self, response: IpcResponse) {
        let _ = self.0.try_send(response);
    }
}

#[derive(Debug, Clone)]
pub struct IpcRequest {
    pub command: IpcCommand,
    pub reply: IpcReply,
}

/// `$XDG_RUNTIME_DIR/ashell.sock`, else a private (0700, ours, not a symlink)
/// per-user dir in the temp dir so other users can't squat or spoof it.
fn socket_path(create_dir: bool) -> Result<PathBuf> {
    Ok(xdg::private_dir(create_dir)?.join("ashell.sock"))
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Run the IPC client: connect to the daemon, send a command, print the response.
pub fn run_client(cmd: &IpcCommand) -> Result<()> {
    let path = socket_path(false).context("locate IPC socket (is ashell running?)")?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("connect to {} — is ashell running?", path.display()))?;

    stream.set_read_timeout(Some(RESPONSE_TIMEOUT + IO_TIMEOUT * 2))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;

    let line = serde_json::to_string(cmd).context("encode IPC command")? + "\n";
    stream.write_all(line.as_bytes()).context("send command")?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    BufReader::new(&stream)
        .read_line(&mut response)
        .context("read response")?;
    match serde_json::from_str(&response).context("decode IPC response")? {
        IpcResponse::Ok {
            notifications: Some(notifications),
        } => println!("{}", serde_json::to_string(&notifications)?),
        IpcResponse::Ok {
            notifications: None,
        } => println!("ok"),
        IpcResponse::Error { message } => return Err(anyhow!(message)),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

enum ListenerError {
    /// Another ashell instance is already listening on the socket.
    AlreadyRunning,
    Other(anyhow::Error),
}

/// Create the Unix listener, taking care not to steal a live server's socket.
///
/// The socket path is shared across instances, so we probe it first: a
/// successful connect means a primary is already serving and we must not
/// remove the file or bind a new listener — otherwise we'd orphan the
/// primary's fd and break `ashell msg` until it's restarted.
fn create_listener() -> std::result::Result<UnixListener, ListenerError> {
    let path = socket_path(true).map_err(ListenerError::Other)?;

    match UnixStream::connect(&path) {
        Ok(_) => return Err(ListenerError::AlreadyRunning),
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            if let Err(e) = std::fs::remove_file(&path)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                return Err(ListenerError::Other(
                    anyhow::Error::new(e)
                        .context(format!("remove stale socket {}", path.display())),
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(ListenerError::Other(
                anyhow::Error::new(e).context(format!("probe socket {}", path.display())),
            ));
        }
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("bind {}", path.display()))
        .map_err(ListenerError::Other)?;
    listener
        .set_nonblocking(true)
        .context("set_nonblocking")
        .map_err(ListenerError::Other)?;
    log::info!("IPC listening on {}", path.display());
    Ok(listener)
}

/// Read a single command from an accepted client connection.
async fn read_request(stream: &mut tokio::net::UnixStream) -> Result<IpcCommand> {
    let mut line = String::new();
    tokio::io::BufReader::new(stream.take(MAX_REQUEST_LEN + 1))
        .read_line(&mut line)
        .await
        .context("read IPC command")?;
    if line.len() as u64 > MAX_REQUEST_LEN {
        return Err(anyhow!("IPC request exceeds {MAX_REQUEST_LEN} bytes"));
    }
    serde_json::from_str(&line).context("decode IPC command")
}

/// Write a response line to the client.
async fn write_response(stream: &mut tokio::net::UnixStream, response: &IpcResponse) {
    let msg = match serde_json::to_string(response) {
        Ok(json) => json + "\n",
        Err(e) => {
            log::error!("IPC encode response failed: {e}");
            return;
        }
    };
    match timeout(IO_TIMEOUT, stream.write_all(msg.as_bytes())).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => log::debug!("IPC write response failed: {e}"),
        Err(_) => log::debug!("IPC write response timed out"),
    }
}

/// Each connection waits for its own app reply without holding up the listener.
async fn handle_connection(mut stream: tokio::net::UnixStream, mut output: Sender<IpcRequest>) {
    let response = if !is_same_user(&stream) {
        log::warn!("IPC: rejected connection from another user");
        IpcResponse::error("permission denied")
    } else {
        match timeout(IO_TIMEOUT, read_request(&mut stream)).await {
            Ok(Ok(command)) => {
                let (reply, mut rx) = IpcReply::channel();
                let request = IpcRequest { command, reply };
                match timeout(RESPONSE_TIMEOUT, async {
                    if output.send(request).await.is_err() {
                        IpcResponse::error("ashell is shutting down")
                    } else {
                        rx.recv()
                            .await
                            .unwrap_or_else(|| IpcResponse::error("IPC request was cancelled"))
                    }
                })
                .await
                {
                    Ok(response) => response,
                    Err(_) => {
                        IpcResponse::error("IPC response timed out; the command may still complete")
                    }
                }
            }
            Ok(Err(e)) => IpcResponse::error(format!("{e:#}")),
            Err(_) => IpcResponse::error("IPC request timed out"),
        }
    };
    write_response(&mut stream, &response).await;
}

/// Defence in depth: the socket dir is already private.
fn is_same_user(stream: &tokio::net::UnixStream) -> bool {
    let uid = unsafe { libc::geteuid() };
    stream.peer_cred().is_ok_and(|cred| cred.uid() == uid)
}

fn init_listener() -> Option<tokio::net::UnixListener> {
    let std_listener = match create_listener() {
        Ok(l) => l,
        Err(ListenerError::AlreadyRunning) => {
            log::warn!(
                "another ashell instance owns the IPC socket; this instance will run without IPC"
            );
            return None;
        }
        Err(ListenerError::Other(e)) => {
            log::error!("Failed to create IPC listener: {e:#}");
            return None;
        }
    };
    match tokio::net::UnixListener::from_std(std_listener) {
        Ok(l) => Some(l),
        Err(e) => {
            log::error!("Failed to convert IPC listener to tokio: {e}");
            None
        }
    }
}

/// Subscription that listens for IPC commands on the Unix socket.
pub fn subscription() -> Subscription<IpcRequest> {
    Subscription::run(|| {
        iced::stream::channel(10, |output: Sender<IpcRequest>| async move {
            let Some(listener) = init_listener() else {
                return;
            };
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    // Finished handlers are reaped even under a steady stream of clients.
                    biased;
                    Some(result) = connections.join_next(), if !connections.is_empty() => {
                        if let Err(e) = result {
                            log::error!("IPC connection task failed: {e}");
                        }
                    }
                    accepted = listener.accept(), if connections.len() < MAX_CONNECTIONS => {
                        match accepted {
                            Ok((stream, _)) => {
                                connections.spawn(handle_connection(stream, output.clone()));
                            }
                            Err(e) => {
                                log::error!("IPC accept error: {e}");
                                return;
                            }
                        }
                    }
                }
            }
        })
    })
}

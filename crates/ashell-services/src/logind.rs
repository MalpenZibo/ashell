use crate::stream::channel;
use futures::{SinkExt, Stream, StreamExt};
use log::warn;
use std::time::Duration;

const RETRY_DELAY: Duration = Duration::from_secs(5);

/// The system woke up from sleep.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResumeEvent;

#[derive(Debug, Clone)]
pub struct Logind {
    conn: zbus::Connection,
}

impl Logind {
    pub async fn connect() -> anyhow::Result<Self> {
        let conn = zbus::Connection::system().await?;

        Ok(Self { conn })
    }

    /// Yields every time the system resumes from sleep. Unlike the other
    /// services' `updates`, there is no state to start with.
    pub fn resumes(&self) -> impl Stream<Item = ResumeEvent> + Send + 'static {
        let conn = self.conn.clone();

        channel(100, |mut output| async move {
            loop {
                let mut signals = match prepare_for_sleep(&conn).await {
                    Ok(signals) => signals,
                    Err(err) => {
                        if conn.is_closed() {
                            warn!("Lost the system bus, no longer watching for resume: {err}");
                            return;
                        }
                        warn!("Failed to subscribe to logind's PrepareForSleep: {err}");
                        tokio::time::sleep(RETRY_DELAY).await;
                        continue;
                    }
                };

                while let Some(signal) = signals.next().await {
                    if let Ok(args) = signal.args()
                        && !args.starting
                        && output.send(ResumeEvent).await.is_err()
                    {
                        return;
                    }
                }
            }
        })
    }
}

async fn prepare_for_sleep(conn: &zbus::Connection) -> zbus::Result<PrepareForSleepStream> {
    Login1ManagerProxy::new(conn)
        .await?
        .receive_prepare_for_sleep()
        .await
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, starting: bool) -> ();
}

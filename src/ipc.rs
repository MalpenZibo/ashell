//! IPC via Unix domain socket.
//!
//! The daemon listens on `$XDG_RUNTIME_DIR/ashell.sock`, or on
//! `$TMPDIR/ashell-<uid>/ashell.sock` when the runtime dir is unusable.
//! The same binary acts as a client via `ashell msg <command>`.

use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::config;
use crate::xdg;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use iced::Subscription;
use serde::Serialize;

use crate::services::notifications::{Notification, Urgency};

use iced::futures::StreamExt;

/// Maximum bytes to read from a client connection.
const MAX_REQUEST_LEN: u64 = 4096;
/// How long the server waits for a connected client to send its command.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// IPC command that can be sent to the daemon.
#[derive(Subcommand, Debug, Clone)]
pub enum IpcCommand {
    /// Toggle bar visibility
    ToggleVisibility,
    VolumeUp {
        #[arg(long)]
        no_osd: bool,
    },
    VolumeDown {
        #[arg(long)]
        no_osd: bool,
    },
    VolumeToggleMute {
        #[arg(long)]
        no_osd: bool,
    },
    MicrophoneUp {
        #[arg(long)]
        no_osd: bool,
    },
    MicrophoneDown {
        #[arg(long)]
        no_osd: bool,
    },
    MicrophoneToggleMute {
        #[arg(long)]
        no_osd: bool,
    },
    BrightnessUp {
        #[arg(long)]
        no_osd: bool,
    },
    BrightnessDown {
        #[arg(long)]
        no_osd: bool,
    },
    ToggleAirplaneMode {
        #[arg(long)]
        no_osd: bool,
    },
    ToggleIdleInhibitor {
        #[arg(long)]
        no_osd: bool,
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
        action: String,
    },
    /// Print retained notifications as a JSON array, newest first
    NotificationsList,
    LoadConfig {
        #[arg(value_parser = config::resolve_config_arg)]
        file: PathBuf,
    },
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

const NO_OSD_SUFFIX: &str = "?no-osd";

impl fmt::Display for IpcCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = match self {
            IpcCommand::ToggleVisibility => "toggle-visibility",
            IpcCommand::VolumeUp { .. } => "volume-up",
            IpcCommand::VolumeDown { .. } => "volume-down",
            IpcCommand::VolumeToggleMute { .. } => "volume-toggle-mute",
            IpcCommand::MicrophoneUp { .. } => "microphone-up",
            IpcCommand::MicrophoneDown { .. } => "microphone-down",
            IpcCommand::MicrophoneToggleMute { .. } => "microphone-toggle-mute",
            IpcCommand::BrightnessUp { .. } => "brightness-up",
            IpcCommand::BrightnessDown { .. } => "brightness-down",
            IpcCommand::ToggleAirplaneMode { .. } => "toggle-airplane-mode",
            IpcCommand::ToggleIdleInhibitor { .. } => "toggle-idle-inhibitor",
            IpcCommand::LoadConfig { .. } => "load-config",
            IpcCommand::NotificationsClear { id } => {
                return match id {
                    Some(id) => write!(f, "notifications-clear {id}"),
                    None => write!(f, "notifications-clear"),
                };
            }
            IpcCommand::NotificationsInvoke { id, action } => {
                let id = id.map_or_else(|| "*".to_string(), |id| id.to_string());
                return write!(f, "notifications-invoke {id} {action}");
            }
            IpcCommand::NotificationsList => "notifications-list",
        };
        write!(f, "{base}")?;
        if let IpcCommand::LoadConfig { file } = self {
            write!(f, " {}", file.display())?;
        }
        if self.no_osd() {
            write!(f, "{NO_OSD_SUFFIX}")?;
        }
        Ok(())
    }
}

impl FromStr for IpcCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let (name, rest) = s.split_once(' ').unwrap_or((s, ""));
        match name {
            "notifications-clear" => {
                let id = if rest.is_empty() {
                    None
                } else {
                    Some(rest.parse()?)
                };
                return Ok(IpcCommand::NotificationsClear { id });
            }
            "notifications-invoke" => {
                let (id, action) = rest
                    .split_once(' ')
                    .filter(|(_, action)| !action.is_empty())
                    .ok_or_else(|| {
                        anyhow!("notifications-invoke requires an ID or * and an action")
                    })?;
                let id = if id == "*" { None } else { Some(id.parse()?) };
                return Ok(IpcCommand::NotificationsInvoke {
                    id,
                    action: action.into(),
                });
            }
            "notifications-list" if rest.is_empty() => return Ok(IpcCommand::NotificationsList),
            _ => {}
        }
        let (cmd, no_osd) = match s.strip_suffix(NO_OSD_SUFFIX) {
            Some(base) => (base, true),
            None => (s, false),
        };

        let mut parts = cmd.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").trim();
        let rest = parts.next().map(str::trim).filter(|s| !s.is_empty());

        match name {
            "toggle-visibility" => Ok(IpcCommand::ToggleVisibility),
            "volume-up" => Ok(IpcCommand::VolumeUp { no_osd }),
            "volume-down" => Ok(IpcCommand::VolumeDown { no_osd }),
            "volume-toggle-mute" => Ok(IpcCommand::VolumeToggleMute { no_osd }),
            "microphone-up" => Ok(IpcCommand::MicrophoneUp { no_osd }),
            "microphone-down" => Ok(IpcCommand::MicrophoneDown { no_osd }),
            "microphone-toggle-mute" => Ok(IpcCommand::MicrophoneToggleMute { no_osd }),
            "brightness-up" => Ok(IpcCommand::BrightnessUp { no_osd }),
            "brightness-down" => Ok(IpcCommand::BrightnessDown { no_osd }),
            "toggle-airplane-mode" => Ok(IpcCommand::ToggleAirplaneMode { no_osd }),
            "toggle-idle-inhibitor" => Ok(IpcCommand::ToggleIdleInhibitor { no_osd }),
            "load-config" => {
                let file =
                    PathBuf::from(rest.ok_or_else(|| anyhow!("load-config requires a file path"))?);
                if !file.is_absolute() {
                    return Err(anyhow!(
                        "load-config expects an absolute path, got {}",
                        file.display()
                    ));
                }
                config::read_config(&file)
                    .map_err(|e| anyhow!("invalid config {}: {e}", file.display()))?;
                Ok(IpcCommand::LoadConfig { file })
            }
            _ => Err(anyhow!("unknown IPC command: {s:?}")),
        }
    }
}

#[derive(Serialize)]
pub struct NotificationInfo<'a> {
    id: u32,
    app_name: &'a str,
    summary: &'a str,
    body: &'a str,
    urgency: &'static str,
    actions: Vec<NotificationAction<'a>>,
}

#[derive(Serialize)]
struct NotificationAction<'a> {
    key: &'a str,
    label: &'a str,
}

impl<'a> From<&'a Notification> for NotificationInfo<'a> {
    fn from(notification: &'a Notification) -> Self {
        Self {
            id: notification.id,
            app_name: &notification.app_name,
            summary: &notification.summary,
            body: &notification.body,
            urgency: match notification.urgency {
                Urgency::Low => "low",
                Urgency::Normal => "normal",
                Urgency::Critical => "critical",
            },
            actions: notification
                .action_pairs()
                .map(|(key, label)| NotificationAction { key, label })
                .collect(),
        }
    }
}

/// Commands are acknowledged on receipt; only listing needs an app response.
#[derive(Debug, Clone)]
pub enum IpcEvent {
    Command(IpcCommand),
    NotificationsList(Arc<UnixStream>),
}

/// Send a notification list response without blocking the app's update loop.
pub async fn respond_notifications_list(stream: Arc<UnixStream>, response: String) {
    if let Err(e) = tokio::task::spawn_blocking(move || {
        write_response(&*stream, &response);
    })
    .await
    {
        log::error!("IPC list response failed: {e}");
    }
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

    let line = format!("{cmd}\n");
    stream.write_all(line.as_bytes()).context("send command")?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    // Notification history can be larger than the command size limit.
    let response_limit = if matches!(cmd, IpcCommand::NotificationsList) {
        u64::MAX
    } else {
        MAX_REQUEST_LEN
    };
    BufReader::new((&stream).take(response_limit))
        .read_line(&mut response)
        .context("read response")?;
    let response = response.trim_end();

    if let Some(err) = response.strip_prefix("error ") {
        return Err(anyhow!("{err}"));
    }

    if !response.is_empty() {
        println!("{response}");
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
fn read_request(stream: &UnixStream) -> Result<IpcCommand> {
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST_LEN))
        .read_line(&mut line)
        .context("read IPC command")?;
    line.trim().parse()
}

/// Write a response line to the client.
fn write_response(mut stream: impl Write, response: &str) {
    let msg = format!("{response}\n");
    if let Err(e) = stream.write_all(msg.as_bytes()) {
        log::debug!("IPC write response failed: {e}");
    }
}

/// Handle a single accepted client connection.
fn handle_connection(mut stream: UnixStream) -> Option<IpcEvent> {
    // tokio's `into_std` leaves the socket nonblocking, so a read that races
    // ahead of the client's write fails with EAGAIN. Block instead, bounded so
    // a client that never writes can't stall the IPC subscription.
    if let Err(e) = stream
        .set_nonblocking(false)
        .and_then(|()| stream.set_read_timeout(Some(REQUEST_TIMEOUT)))
    {
        log::debug!("IPC: failed to make stream blocking: {e}");
    }

    match read_request(&stream) {
        Ok(IpcCommand::NotificationsList) => Some(IpcEvent::NotificationsList(Arc::new(stream))),
        Ok(cmd) => {
            write_response(&mut stream, "ok");
            Some(IpcEvent::Command(cmd))
        }
        Err(e) => {
            write_response(&mut stream, &format!("error {e:#}"));
            None
        }
    }
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
pub fn subscription() -> Subscription<IpcEvent> {
    Subscription::run(|| {
        iced::futures::stream::unfold(None::<tokio::net::UnixListener>, |listener| async {
            let listener = match listener {
                Some(l) => l,
                None => init_listener()?,
            };
            let (request, listener) = match listener.accept().await {
                Ok((stream, _)) => {
                    let same_user = is_same_user(&stream);
                    let request = match stream.into_std() {
                        Ok(mut std_stream) if !same_user => {
                            log::warn!("IPC: rejected connection from another user");
                            write_response(&mut std_stream, "error permission denied");
                            None
                        }
                        Ok(std_stream) => handle_connection(std_stream),
                        Err(e) => {
                            log::error!("IPC stream conversion error: {e}");
                            None
                        }
                    };
                    (request, listener)
                }
                Err(e) => {
                    log::error!("IPC accept error: {e}");
                    (None, listener)
                }
            };
            Some((request, Some(listener)))
        })
        .filter_map(iced::futures::future::ready)
    })
}

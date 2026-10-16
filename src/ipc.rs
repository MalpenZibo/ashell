//! IPC via Unix domain socket.
//!
//! The daemon listens on `$XDG_RUNTIME_DIR/ashell.sock`, or on
//! `$TMPDIR/ashell-<uid>/ashell.sock` when the runtime dir is unusable.
//! The same binary acts as a client via `ashell msg <command>`.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use crate::config;
use crate::xdg;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use iced::Subscription;
use serde::{Deserialize, Serialize};

use iced::futures::StreamExt;

/// Maximum bytes to read from a client connection.
const MAX_REQUEST_LEN: u64 = 4096;
/// How long the server waits for a connected client to send its command.
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

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
            IpcCommand::LoadConfig { .. } => false,
        }
    }
}

/// A newline-delimited JSON reply to a socket request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum IpcResponse {
    Ok,
    Error { message: String },
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

    let line = serde_json::to_string(cmd).context("encode IPC command")? + "\n";
    stream.write_all(line.as_bytes()).context("send command")?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    BufReader::new(&stream)
        .read_line(&mut response)
        .context("read response")?;
    match serde_json::from_str(&response).context("decode IPC response")? {
        IpcResponse::Ok => println!("ok"),
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
fn read_request(stream: &UnixStream) -> Result<IpcCommand> {
    let mut line = String::new();
    BufReader::new(stream.take(MAX_REQUEST_LEN + 1))
        .read_line(&mut line)
        .context("read IPC command")?;
    if line.len() as u64 > MAX_REQUEST_LEN {
        return Err(anyhow!("IPC request exceeds {MAX_REQUEST_LEN} bytes"));
    }
    let cmd = serde_json::from_str(&line).context("decode IPC command")?;
    if let IpcCommand::LoadConfig { file } = &cmd {
        if !file.is_absolute() {
            return Err(anyhow!(
                "load-config expects an absolute path, got {}",
                file.display()
            ));
        }
        config::read_config(file).map_err(|e| anyhow!("invalid config {}: {e}", file.display()))?;
    }
    Ok(cmd)
}

/// Write a response line to the client.
fn write_response(stream: &mut UnixStream, response: &IpcResponse) {
    let msg = match serde_json::to_string(response) {
        Ok(json) => json + "\n",
        Err(e) => {
            log::error!("IPC encode response failed: {e}");
            return;
        }
    };
    if let Err(e) = stream.write_all(msg.as_bytes()) {
        log::debug!("IPC write response failed: {e}");
    }
}

/// Handle a single accepted client connection.
fn handle_connection(mut stream: UnixStream) -> Option<IpcCommand> {
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
        Ok(cmd) => {
            write_response(&mut stream, &IpcResponse::Ok);
            Some(cmd)
        }
        Err(e) => {
            write_response(
                &mut stream,
                &IpcResponse::Error {
                    message: format!("{e:#}"),
                },
            );
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
pub fn subscription() -> Subscription<IpcCommand> {
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
                            write_response(
                                &mut std_stream,
                                &IpcResponse::Error {
                                    message: "permission denied".into(),
                                },
                            );
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

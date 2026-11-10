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

use crate::config;
use crate::xdg;
use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use iced::Subscription;

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
    /// Show (`true`) or hide (`false`) the bar
    SetVisibility {
        // `bool` fields default to `ArgAction::SetTrue`, which is invalid for
        // a positional; make it a required value explicitly.
        #[arg(value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set)]
        state: bool,
    },
    /// Mute (`true`) or unmute (`false`) the sink
    VolumeSetMute {
        // `bool` fields default to `ArgAction::SetTrue`, which is invalid for
        // a positional; make it a required value explicitly.
        #[arg(value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set)]
        state: bool,
        #[arg(long)]
        no_osd: bool,
    },
    /// Mute (`true`) or unmute (`false`) the source
    MicrophoneSetMute {
        // `bool` fields default to `ArgAction::SetTrue`, which is invalid for
        // a positional; make it a required value explicitly.
        #[arg(value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set)]
        state: bool,
        #[arg(long)]
        no_osd: bool,
    },
    /// Turn airplane mode on (`true`) or off (`false`)
    SetAirplaneMode {
        // `bool` fields default to `ArgAction::SetTrue`, which is invalid for
        // a positional; make it a required value explicitly.
        #[arg(value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set)]
        state: bool,
        #[arg(long)]
        no_osd: bool,
    },
    /// Turn the idle inhibitor on (`true`) or off (`false`)
    SetIdleInhibitor {
        // `bool` fields default to `ArgAction::SetTrue`, which is invalid for
        // a positional; make it a required value explicitly.
        #[arg(value_parser = clap::builder::BoolishValueParser::new(), action = clap::ArgAction::Set)]
        state: bool,
        #[arg(long)]
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
            IpcCommand::ToggleVisibility
            | IpcCommand::SetVisibility { .. }
            | IpcCommand::LoadConfig { .. } => false,
            IpcCommand::VolumeUp { no_osd }
            | IpcCommand::VolumeDown { no_osd }
            | IpcCommand::VolumeToggleMute { no_osd }
            | IpcCommand::MicrophoneUp { no_osd }
            | IpcCommand::MicrophoneDown { no_osd }
            | IpcCommand::MicrophoneToggleMute { no_osd }
            | IpcCommand::BrightnessUp { no_osd }
            | IpcCommand::BrightnessDown { no_osd }
            | IpcCommand::ToggleAirplaneMode { no_osd }
            | IpcCommand::ToggleIdleInhibitor { no_osd }
            | IpcCommand::VolumeSetMute { no_osd, .. }
            | IpcCommand::MicrophoneSetMute { no_osd, .. }
            | IpcCommand::SetAirplaneMode { no_osd, .. }
            | IpcCommand::SetIdleInhibitor { no_osd, .. } => *no_osd,
        }
    }

    /// The forced state of the `set-*` commands, if any.
    fn state(&self) -> Option<bool> {
        match self {
            IpcCommand::SetVisibility { state }
            | IpcCommand::VolumeSetMute { state, .. }
            | IpcCommand::MicrophoneSetMute { state, .. }
            | IpcCommand::SetAirplaneMode { state, .. }
            | IpcCommand::SetIdleInhibitor { state, .. } => Some(*state),
            _ => None,
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
            IpcCommand::SetVisibility { .. } => "set-visibility",
            IpcCommand::VolumeSetMute { .. } => "volume-set-mute",
            IpcCommand::MicrophoneSetMute { .. } => "microphone-set-mute",
            IpcCommand::SetAirplaneMode { .. } => "set-airplane-mode",
            IpcCommand::SetIdleInhibitor { .. } => "set-idle-inhibitor",
            IpcCommand::LoadConfig { .. } => "load-config",
        };
        write!(f, "{base}")?;
        if let IpcCommand::LoadConfig { file } = self {
            write!(f, " {}", file.display())?;
        }
        if let Some(state) = self.state() {
            write!(f, " {state}")?;
        }
        if self.no_osd() {
            write!(f, "{NO_OSD_SUFFIX}")?;
        }
        Ok(())
    }
}

/// Parse the required `true`/`false` argument of the `set-*` commands.
fn parse_bool(name: &str, value: Option<&str>) -> Result<bool> {
    let Some(value) = value else {
        return Err(anyhow!("{name} requires a true or false argument"));
    };

    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(anyhow!("{name} expects true or false, got {value:?}")),
    }
}

impl FromStr for IpcCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let (cmd, no_osd) = match s.strip_suffix(NO_OSD_SUFFIX) {
            Some(base) => (base, true),
            None => (s, false),
        };

        let mut parts = cmd.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").trim();
        let rest = parts.next().map(str::trim).filter(|s| !s.is_empty());

        // Toggles flip on their own; a forced state goes through `set-*`.
        if let Some(rest) = rest
            && matches!(
                name,
                "toggle-visibility"
                    | "volume-toggle-mute"
                    | "microphone-toggle-mute"
                    | "toggle-airplane-mode"
                    | "toggle-idle-inhibitor"
            )
        {
            return Err(anyhow!(
                "{name} takes no arguments, got {rest:?}; use the matching set-* command to force a state"
            ));
        }

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
            "set-visibility" => Ok(IpcCommand::SetVisibility {
                state: parse_bool(name, rest)?,
            }),
            "volume-set-mute" => Ok(IpcCommand::VolumeSetMute {
                state: parse_bool(name, rest)?,
                no_osd,
            }),
            "microphone-set-mute" => Ok(IpcCommand::MicrophoneSetMute {
                state: parse_bool(name, rest)?,
                no_osd,
            }),
            "set-airplane-mode" => Ok(IpcCommand::SetAirplaneMode {
                state: parse_bool(name, rest)?,
                no_osd,
            }),
            "set-idle-inhibitor" => Ok(IpcCommand::SetIdleInhibitor {
                state: parse_bool(name, rest)?,
                no_osd,
            }),
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
    BufReader::new((&stream).take(MAX_REQUEST_LEN))
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
fn write_response(stream: &mut UnixStream, response: &str) {
    let msg = format!("{response}\n");
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
            write_response(&mut stream, "ok");
            Some(cmd)
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
                            write_response(&mut std_stream, "error permission denied");
                            None
                        }
                        Ok(std_stream) => {
                            // `handle_connection` reads with a bounded timeout;
                            // keep that blocking work off the async executor.
                            tokio::task::spawn_blocking(move || handle_connection(std_stream))
                                .await
                                .unwrap_or_else(|e| {
                                    log::error!("IPC connection task failed: {e}");
                                    None
                                })
                        }
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

#[cfg(test)]
mod tests {
    use super::IpcCommand;

    fn parse(command: &str) -> IpcCommand {
        command.parse().unwrap()
    }

    #[test]
    fn set_commands_round_trip() {
        for command in [
            "set-visibility true",
            "set-visibility false",
            "volume-set-mute true?no-osd",
            "microphone-set-mute false",
            "set-airplane-mode true",
            "set-idle-inhibitor false",
        ] {
            assert_eq!(parse(command).to_string(), command);
        }
    }

    #[test]
    fn toggles_remain_parameterless() {
        for command in [
            "toggle-visibility",
            "volume-toggle-mute",
            "microphone-toggle-mute?no-osd",
            "toggle-airplane-mode",
            "toggle-idle-inhibitor",
        ] {
            let parsed = parse(command);
            assert_eq!(parsed.state(), None);
            assert_eq!(parsed.to_string(), command);
        }
    }

    #[test]
    fn boolish_state_spellings_normalise() {
        for (command, expected) in [
            ("set-visibility yes", true),
            ("set-visibility 1", true),
            ("set-visibility no", false),
            ("set-visibility 0", false),
        ] {
            let parsed = parse(command);
            assert_eq!(parsed.state(), Some(expected));
            assert_eq!(parsed.to_string(), format!("set-visibility {expected}"));
        }
    }

    #[test]
    fn rejects_a_state_that_is_not_a_bool() {
        assert!("set-visibility maybe".parse::<IpcCommand>().is_err());
        assert!("set-airplane-mode 2".parse::<IpcCommand>().is_err());
    }

    #[test]
    fn set_commands_require_a_state() {
        for command in [
            "set-visibility",
            "volume-set-mute",
            "microphone-set-mute",
            "set-airplane-mode",
            "set-idle-inhibitor",
        ] {
            assert!(command.parse::<IpcCommand>().is_err(), "{command}");
        }
    }

    #[test]
    fn toggles_reject_arguments() {
        assert!("toggle-visibility true".parse::<IpcCommand>().is_err());
        assert!("toggle-airplane-mode false".parse::<IpcCommand>().is_err());
    }

    #[test]
    fn state_and_no_osd_compose() {
        let parsed = parse("set-airplane-mode false?no-osd");
        assert_eq!(parsed.state(), Some(false));
        assert!(parsed.no_osd());
        assert_eq!(parsed.to_string(), "set-airplane-mode false?no-osd");
    }

    #[test]
    fn clap_accepts_the_set_commands() {
        use clap::Parser;

        for (args, expected) in [
            (
                vec!["ashell", "msg", "set-visibility", "false"],
                "set-visibility false",
            ),
            (
                vec!["ashell", "msg", "toggle-visibility"],
                "toggle-visibility",
            ),
            (
                vec!["ashell", "msg", "volume-set-mute", "true", "--no-osd"],
                "volume-set-mute true?no-osd",
            ),
            (
                vec!["ashell", "msg", "set-airplane-mode", "--no-osd", "off"],
                "set-airplane-mode false?no-osd",
            ),
        ] {
            let args = crate::Args::try_parse_from(args).unwrap();
            let Some(crate::Command::Msg { command }) = args.command else {
                panic!("expected the msg subcommand");
            };
            assert_eq!(command.to_string(), expected);
        }
    }
}

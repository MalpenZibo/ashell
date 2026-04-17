//! IPC via Unix domain socket.
//!
//! The daemon listens on `$XDG_RUNTIME_DIR/ashell.sock`.
//! The same binary acts as a client via `ashell msg <command>`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};

use crate::IpcCommand;

/// Resolve the socket path.
pub fn socket_path() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return Ok(PathBuf::from(dir).join("ashell.sock"));
    }
    let uid = unsafe { libc::getuid() };
    Ok(PathBuf::from(format!("/tmp/ashell-{uid}.sock")))
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Run the IPC client: connect to the daemon, send a command, print the response.
pub fn run_client(cmd: &IpcCommand) -> Result<()> {
    let path = socket_path()?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("connect to {} — is ashell running?", path.display()))?;

    let line = match cmd {
        IpcCommand::ToggleVisibility => "toggle-visibility\n",
    };
    stream.write_all(line.as_bytes()).context("send command")?;
    stream.flush()?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut response = String::new();
    BufReader::new(&stream)
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

/// Parsed IPC request from a client.
pub enum IpcRequest {
    ToggleVisibility,
}

/// Create the Unix listener, removing any stale socket file first.
pub fn create_listener() -> Result<UnixListener> {
    let path = socket_path()?;

    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("remove stale socket {}", path.display()))?;
    }

    let listener = UnixListener::bind(&path).with_context(|| format!("bind {}", path.display()))?;
    listener.set_nonblocking(true).context("set_nonblocking")?;
    log::info!("IPC listening on {}", path.display());
    Ok(listener)
}

/// Read a single command from an accepted client connection.
pub fn read_request(stream: &UnixStream) -> Result<IpcRequest> {
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .context("read IPC command")?;
    let line = line.trim();

    match line {
        "toggle-visibility" => Ok(IpcRequest::ToggleVisibility),
        _ => Err(anyhow!("unknown IPC command: {line:?}")),
    }
}

/// Write a success response to the client.
pub fn write_response(stream: &mut UnixStream, response: &str) {
    let msg = format!("{response}\n");
    if let Err(e) = stream.write_all(msg.as_bytes()) {
        log::debug!("IPC write response failed: {e}");
    }
}

/// Write an error response to the client.
pub fn write_error(stream: &mut UnixStream, err: &str) {
    write_response(stream, &format!("error {err}"));
}

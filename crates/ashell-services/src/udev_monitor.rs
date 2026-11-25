use tokio::io::{Interest, unix::AsyncFd};

/// Listens for the udev events of `subsystem`.
pub(crate) fn listen(subsystem: &str) -> anyhow::Result<AsyncFd<udev::MonitorSocket>> {
    let socket = udev::MonitorBuilder::new()?
        .match_subsystem(subsystem)?
        .listen()?;

    // SAFETY: the socket owns its udev monitor fd until it is dropped,
    // and `as_raw_fd` always returns that same fd.
    Ok(unsafe { AsyncFd::register_with_interest(socket, Interest::READABLE) }?)
}

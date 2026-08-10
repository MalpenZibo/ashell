use std::env;
use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

// $XDG_RUNTIME_DIR defines the base directory relative to which user-specific non-essential runtime
// files and other file objects (such as sockets, named pipes, ...) should be stored.
// The directory MUST be owned by the user, and they MUST be the only one having read and write
// access to it. Its Unix access mode MUST be 0700.
pub fn get_runtime_dir() -> Option<PathBuf> {
    let runtime_dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    let metadata = runtime_dir.metadata().ok()?;
    let uid = unsafe { libc::geteuid() };
    (runtime_dir.is_absolute()
        && metadata.is_dir()
        && metadata.st_uid() == uid
        && metadata.permissions().mode() & 0o777 == 0o700)
        .then_some(runtime_dir)
}

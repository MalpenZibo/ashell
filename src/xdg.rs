use std::env;
use std::fs::{DirBuilder, Metadata};
use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::Once;

use anyhow::{Context, Result, anyhow};

// $XDG_RUNTIME_DIR defines the base directory relative to which user-specific non-essential runtime
// files and other file objects (such as sockets, named pipes, ...) should be stored.
// The directory MUST be owned by the user, and they MUST be the only one having read and write
// access to it. Its Unix access mode MUST be 0700.
pub fn get_runtime_dir() -> Option<PathBuf> {
    let runtime_dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR")?);
    let metadata = runtime_dir.metadata().ok()?;
    (runtime_dir.is_absolute() && is_private_dir(&metadata)).then_some(runtime_dir)
}

/// A directory owned by the effective user with mode 0700.
pub fn is_private_dir(metadata: &Metadata) -> bool {
    let uid = unsafe { libc::geteuid() };
    metadata.is_dir() && metadata.st_uid() == uid && metadata.permissions().mode() & 0o777 == 0o700
}

static FALLBACK_WARNED: Once = Once::new();

/// A private directory for ashell's own files: `$XDG_RUNTIME_DIR` when it
/// validates, else `$TMPDIR/ashell-<uid>`. With `create`, the fallback
/// directory is created with mode 0700. Either way it must be a real
/// directory owned by the effective user with mode 0700, so other users
/// can neither squat on the path nor plant files inside it.
pub fn private_dir(create: bool) -> Result<PathBuf> {
    if let Some(dir) = get_runtime_dir() {
        return Ok(dir);
    }

    let uid = unsafe { libc::geteuid() };
    let dir = env::temp_dir().join(format!("ashell-{uid}"));
    if create {
        FALLBACK_WARNED.call_once(|| {
            log::warn!(
                "XDG_RUNTIME_DIR is unset or invalid, falling back to {} for ashell's runtime files",
                dir.display()
            );
        });
        match DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => {
                return Err(anyhow::Error::new(e).context(format!("create {}", dir.display())));
            }
        }
    }

    let metadata =
        std::fs::symlink_metadata(&dir).with_context(|| format!("stat {}", dir.display()))?;
    if !is_private_dir(&metadata) {
        return Err(anyhow!(
            "{} is not a directory owned by you with mode 0700",
            dir.display()
        ));
    }
    Ok(dir)
}

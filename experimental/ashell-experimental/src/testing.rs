//! What the tests share.

use std::ops::Deref;
use std::path::{Path, PathBuf};

/// A folder of a test's own under the temporary directory, empty when made
/// and removed with it, however the test ends.
pub struct Folder(PathBuf);

impl Folder {
    /// `name` keeps the tests' folders apart, the process's id the runs'.
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("ashell-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();

        Self(std::fs::canonicalize(path).unwrap())
    }

    pub fn write(&self, name: &str, text: &str) -> &Self {
        std::fs::write(self.join(name), text).unwrap();

        self
    }
}

impl Deref for Folder {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

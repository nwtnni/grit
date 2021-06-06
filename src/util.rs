use std::borrow;
use std::cmp;
use std::os::unix::ffi::OsStrExt as _;
use std::path;

pub trait Tap: Sized {
    fn tap<F: FnOnce(Self) -> T, T>(self, apply: F) -> T {
        apply(self)
    }

    fn tap_mut<F: FnOnce(&mut Self)>(mut self, apply: F) -> Self {
        apply(&mut self);
        self
    }
}

impl<T: Sized> Tap for T {}

/// Wrapper around `std::path::PathBuf` that compares paths byte-wise (including the
/// directory separator) as opposed to component-wise. This matches how `git` stores
/// and orders paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathBuf(pub path::PathBuf);

impl borrow::Borrow<path::Path> for PathBuf {
    fn borrow(&self) -> &path::Path {
        self.0.borrow()
    }
}

impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathBuf {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0
            .as_os_str()
            .as_bytes()
            .cmp(other.0.as_os_str().as_bytes())
    }
}

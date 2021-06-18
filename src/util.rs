use std::borrow;
use std::cmp;
use std::hash;
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

// See: http://idubrov.name/rust/2018/06/01/tricking-the-hashmap.html
// and: https://github.com/sunshowers/borrow-complex-key-example
pub trait Key {
    fn key(&self) -> Path;
}

impl<'a> Eq for dyn Key + 'a {}

impl<'a> hash::Hash for dyn Key + 'a {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.key().hash(state)
    }
}

impl<'a> PartialEq for dyn Key + 'a {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl<'a> PartialOrd for dyn Key + 'a {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for dyn Key + 'a {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// Wrapper around `std::path::PathBuf` that compares paths byte-wise (including the
/// directory separator) as opposed to component-wise. This matches how `git` stores
/// and orders paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathBuf(pub path::PathBuf);

impl Key for PathBuf {
    fn key(&self) -> Path {
        Path(self.0.as_path())
    }
}

impl<'a> borrow::Borrow<dyn Key + 'a> for PathBuf {
    fn borrow(&self) -> &(dyn Key + 'a) {
        self
    }
}

impl PartialOrd for PathBuf {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathBuf {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.key().cmp(&other.key())
    }
}

/// Wrapper around `std::path::Path` that compares paths byte-wise (including the
/// directory separator) as opposed to component-wise. This matches how `git` stores
/// and orders paths.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Path<'a>(pub &'a path::Path);

impl<'a> Key for Path<'a> {
    fn key(&self) -> Path {
        *self
    }
}

impl<'a> PartialOrd for Path<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Path<'a> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0
            .as_os_str()
            .as_bytes()
            .cmp(other.0.as_os_str().as_bytes())
    }
}

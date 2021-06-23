use std::path;

use crate::util::Tap as _;

#[derive(Debug)]
pub struct Workspace {
    root: path::PathBuf,
}

impl Workspace {
    pub fn new(root: path::PathBuf) -> Self {
        Workspace { root }
    }

    pub fn root(&self) -> &path::Path {
        &self.root
    }

    pub fn walk<'a, F: FnOnce(walkdir::WalkDir) -> walkdir::WalkDir>(
        &'a self,
        relative: &path::Path,
        configure: F,
    ) -> impl Iterator<Item = walkdir::Result<DirEntry<'a>>> {
        let root = self.root.join(relative);
        let git = self.root.join(".git");
        walkdir::WalkDir::new(root)
            .tap(configure)
            .into_iter()
            .filter_entry(move |entry| !entry.path().starts_with(&git))
            .map(move |entry| {
                entry.map(|entry| DirEntry {
                    root: self.root.as_path(),
                    inner: entry,
                })
            })
    }
}

#[derive(Clone, Debug)]
pub struct DirEntry<'a> {
    root: &'a path::Path,
    inner: walkdir::DirEntry,
}

impl<'a> std::ops::Deref for DirEntry<'a> {
    type Target = walkdir::DirEntry;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DirEntry<'a> {
    pub fn relative(&self) -> &path::Path {
        self.inner
            .path()
            .strip_prefix(self.root)
            .expect("[INTERNAL ERROR]: workspace must contain path")
    }
}

impl<'a> From<DirEntry<'a>> for walkdir::DirEntry {
    fn from(entry: DirEntry<'a>) -> Self {
        entry.inner
    }
}

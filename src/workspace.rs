use std::path;
use std::rc::Rc;

use crate::util::Tap as _;

#[derive(Debug)]
pub struct Workspace {
    root: Rc<path::Path>,
}

impl Workspace {
    pub fn new(root: path::PathBuf) -> Self {
        Workspace {
            root: Rc::from(root),
        }
    }

    pub fn root(&self) -> &path::Path {
        &self.root
    }

    pub fn walk<F: FnOnce(walkdir::WalkDir) -> walkdir::WalkDir>(
        &self,
        relative: &path::Path,
        configure: F,
    ) -> impl Iterator<Item = walkdir::Result<DirEntry>> {
        let root = Rc::clone(&self.root);
        let path = self.root.join(relative);
        let git = self.root.join(".git");
        walkdir::WalkDir::new(path)
            .tap(configure)
            .into_iter()
            .filter_entry(move |entry| !entry.path().starts_with(&git))
            .map(move |entry| {
                entry.map(|entry| DirEntry {
                    root: Rc::clone(&root),
                    inner: entry,
                })
            })
    }
}

#[derive(Clone, Debug)]
pub struct DirEntry {
    root: Rc<path::Path>,
    inner: walkdir::DirEntry,
}

impl std::ops::Deref for DirEntry {
    type Target = walkdir::DirEntry;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DirEntry {
    pub fn relative(&self) -> &path::Path {
        self.inner
            .path()
            .strip_prefix(&*self.root)
            .expect("[INTERNAL ERROR]: workspace must contain path")
    }
}

impl From<DirEntry> for walkdir::DirEntry {
    fn from(entry: DirEntry) -> Self {
        entry.inner
    }
}

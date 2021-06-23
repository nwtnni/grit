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

    pub fn walk<F: FnOnce(walkdir::WalkDir) -> walkdir::WalkDir>(
        &self,
        relative: &path::Path,
        configure: F,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
        let root = self.root.join(relative);
        let git = self.root.join(".git");
        walkdir::WalkDir::new(root)
            .tap(configure)
            .into_iter()
            .filter_entry(move |entry| !entry.path().starts_with(&git))
    }
}

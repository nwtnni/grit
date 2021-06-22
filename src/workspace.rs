use std::path;

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

    pub fn walk<P: AsRef<path::Path>>(
        &self,
        relative: P,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
        let root = self.root.join(relative);
        let git = self.root.join(".git");
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(move |entry| !entry.path().starts_with(&git))
    }
}

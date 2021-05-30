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
}

impl IntoIterator for &'_ Workspace {
    type IntoIter = Box<dyn Iterator<Item = Self::Item>>;
    type Item = walkdir::Result<walkdir::DirEntry>;
    fn into_iter(self) -> Self::IntoIter {
        let git = self.root.join(".git");
        walkdir::WalkDir::new(&self.root)
            .contents_first(true)
            .sort_by_file_name()
            .into_iter()
            .filter(move |entry| match entry {
                Ok(entry) => !entry.path().starts_with(&git),
                Err(_) => true,
            })
            .tap(Box::new)
    }
}

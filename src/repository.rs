use std::fs;
use std::path;

#[derive(Clone, Debug)]
pub struct Repository {
    root: path::PathBuf,
}

impl Repository {
    pub fn new(root: path::PathBuf) -> Self {
        Repository { root }
    }

    pub fn root(&self) -> &path::Path {
        &self.root
    }

    pub fn database(&self) -> crate::Database {
        crate::Database::new(self.root.join(".git/objects"))
    }

    pub fn index(&self) -> anyhow::Result<crate::Index> {
        crate::Index::lock(self.root.join(".git/index"))
    }

    pub fn references(&self) -> crate::References {
        crate::References::new(self.root.join(".git/refs"), self.root.join(".git/HEAD"))
    }

    pub fn workspace(&self) -> crate::Workspace {
        crate::Workspace::new(self.root.clone())
    }

    pub fn init(&mut self) -> anyhow::Result<()> {
        self.root.push(".git");
        for directory in &["objects", "refs"] {
            self.root.push(directory);
            fs::create_dir_all(&self.root)?;
            self.root.pop();
        }
        self.root.pop();
        Ok(())
    }
}

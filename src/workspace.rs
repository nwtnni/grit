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

    pub fn files(&self) -> impl Iterator<Item = anyhow::Result<path::PathBuf>> {
        walkdir::WalkDir::new(&self.root)
            .max_depth(1)
            .into_iter()
            .filter_map(|entry| match entry {
                Err(error) => Some(Err(anyhow::Error::from(error))),
                Ok(entry) if entry.file_type().is_file() => Some(Ok(entry.into_path())),
                Ok(_) => None,
            })
    }
}

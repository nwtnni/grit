use std::convert;
use std::env;
use std::fs;
use std::path;

use structopt::StructOpt;

use crate::object;

#[derive(StructOpt)]
pub struct Add {
    paths: Vec<path::PathBuf>,
}

impl Add {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let git = root.join(".git");

        let workspace = crate::Workspace::new(root);
        let database = crate::Database::new(&git)?;
        let mut index = crate::Index::lock(&git)?;

        for path in self.paths {
            for entry in workspace.walk(&path, convert::identity) {
                let entry = entry?;

                if entry.file_type().is_dir() {
                    continue;
                }

                let meta = entry.metadata()?;
                let blob = fs::read(entry.path())
                    .map(object::Blob::new)
                    .map(crate::Object::Blob)?;

                let id = database.store(&blob)?;
                let relative = entry
                    .into_path()
                    .strip_prefix(workspace.root())
                    .expect("[UNREACHABLE]: entry must be inside workspace")
                    .to_path_buf();

                index.insert(&meta, id, relative);
            }
        }

        index.commit()?;
        Ok(())
    }
}

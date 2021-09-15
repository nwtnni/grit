use std::env;
use std::fs;
use std::path;

use structopt::StructOpt;

use crate::object;

#[derive(StructOpt)]
pub struct Configuration {
    paths: Vec<path::PathBuf>,
}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let repository = crate::Repository::new(root);
        let add = Add {
            database: repository.database(),
            index: repository.index()?,
            workspace: repository.workspace(),
            paths: self.paths,
        };
        add.run()?;
        Ok(())
    }
}

struct Add {
    database: crate::Database,
    index: crate::Index,
    workspace: crate::Workspace,
    paths: Vec<path::PathBuf>,
}

impl Add {
    fn run(mut self) -> anyhow::Result<()> {
        for path in self.paths {
            for entry in self.workspace.walk_tree(&path)? {
                let entry = entry?;
                let relative = entry.relative_path();

                if entry.metadata.mode.is_directory() {
                    continue;
                }

                let blob = fs::read(entry.path())
                    .map(object::Blob::new)
                    .map(crate::Object::Blob)?;

                let id = self.database.store(&blob)?;

                self.index
                    .insert(entry.metadata, id, relative.to_path_buf());
            }
        }

        self.index.commit()?;
        Ok(())
    }
}

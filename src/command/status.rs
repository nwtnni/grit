use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env;
use std::path;

use structopt::StructOpt;

use crate::meta;
use crate::util;
use crate::util::Tap as _;
use crate::workspace;

#[derive(StructOpt)]
pub struct Configuration {}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let repository = crate::Repository::new(root);
        let status = Status {
            index: repository.index()?,
            workspace: repository.workspace(),
            tracked: BTreeMap::new(),
            untracked: BTreeSet::new(),
        };
        status.run()?;
        Ok(())
    }
}

struct Status {
    index: crate::Index,
    workspace: crate::Workspace,
    tracked: BTreeMap<util::PathBuf, meta::Metadata>,
    untracked: BTreeSet<util::PathBuf>,
}

impl Status {
    fn run(mut self) -> anyhow::Result<()> {
        self.walk_fs(path::Path::new("."))?;
        self.walk_index();
        for util::PathBuf(path) in &self.untracked {
            println!("?? {}", path.display());
        }
        Ok(())
    }

    fn walk_fs(&mut self, relative: &path::Path) -> anyhow::Result<()> {
        for entry in self.workspace.walk_list(relative)? {
            let entry = entry?;
            let relative = entry.relative_path();
            let metadata = entry.metadata;

            match self.index.contains_key(relative) {
                true if metadata.mode.is_directory() => self.walk_fs(relative)?,
                true => {
                    self.tracked
                        .insert(relative.to_path_buf().tap(util::PathBuf), metadata);
                }
                false if self.is_trackable(&entry)? => {
                    let relative = if metadata.mode.is_directory() {
                        relative
                            .as_os_str()
                            .to_os_string()
                            .tap_mut(|path| path.push("/"))
                            .tap(path::PathBuf::from)
                    } else {
                        relative.to_path_buf()
                    };

                    self.untracked.insert(util::PathBuf(relative));
                }
                false => continue,
            }
        }
        Ok(())
    }

    fn walk_index(&self) {
        for entry in self.index.files() {
            let metadata = match self.tracked.get(&entry.path() as &dyn util::Key) {
                Some(metadata) => metadata,
                None => {
                    println!(" D {}", entry.path().display());
                    continue;
                }
            };

            let old = entry.metadata();
            let new = metadata;

            if new.mode != old.mode || new.size != old.size {
                println!(" M {}", entry.path().display());
            }
        }
    }

    fn is_trackable(&self, entry: &workspace::Entry) -> anyhow::Result<bool> {
        let relative = entry.relative_path();

        if entry.metadata().mode.is_file() {
            return Ok(!self.index.contains_key(relative));
        }

        for entry in self.workspace.walk_list(relative)? {
            if self.is_trackable(&entry?)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

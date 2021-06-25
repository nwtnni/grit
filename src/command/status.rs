use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::convert::TryFrom as _;
use std::env;
use std::io;
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
    tracked: BTreeMap<util::PathBuf, meta::Data>,
    untracked: BTreeSet<util::PathBuf>,
}

impl Status {
    fn run(mut self) -> io::Result<()> {
        self.walk_fs(path::Path::new("."))?;
        self.walk_index();
        for util::PathBuf(path) in &self.untracked {
            println!("?? {}", path.display());
        }
        Ok(())
    }

    fn walk_fs(&mut self, relative: &path::Path) -> io::Result<()> {
        for entry in self
            .workspace
            .walk(relative, |walkdir| walkdir.min_depth(1).max_depth(1))
        {
            let entry = entry?;
            let relative = entry.relative();
            let file_type = entry.file_type();
            let metadata = entry
                .metadata()?
                .tap(|metadata| meta::Data::try_from(&metadata))
                .expect("[INTERNAL ERROR]: could not convert metadata");

            match self.index.contains_key(relative) {
                true if file_type.is_dir() => self.walk_fs(relative)?,
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
            let meta = match self
                .tracked
                .get(&util::Path(entry.path()) as &dyn util::Key)
            {
                Some(meta) => meta,
                None => {
                    println!(" D {}", entry.path().display());
                    continue;
                }
            };

            let old = entry.metadata();
            let new = meta;

            if new.mode != old.mode || new.size != old.size {
                println!(" M {}", entry.path().display());
            }
        }
    }

    fn is_trackable(&self, entry: &workspace::DirEntry) -> io::Result<bool> {
        let relative = entry.relative();
        let file_type = entry.file_type();

        if file_type.is_file() {
            return Ok(!self.index.contains_key(relative));
        }

        if file_type.is_symlink() {
            unimplemented!();
        }

        for entry in self.workspace.walk(relative, |walkdir| {
            walkdir
                .min_depth(1)
                .max_depth(1)
                .sort_by_key(|entry| entry.file_type().is_dir())
        }) {
            if self.is_trackable(&entry?)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

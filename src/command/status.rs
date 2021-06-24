use std::env;
use std::io;
use std::path;

use structopt::StructOpt;

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
        };
        status.run()?;
        Ok(())
    }
}

struct Status {
    index: crate::Index,
    workspace: crate::Workspace,
}

impl Status {
    fn run(self) -> io::Result<()> {
        self.walk(path::Path::new("."), &mut |entry| {
            if entry.file_type().is_dir() {
                println!("?? {}/", entry.relative().display());
            } else {
                println!("?? {}", entry.relative().display());
            }
        })
    }

    fn walk<F: for<'a> FnMut(workspace::DirEntry<'a>)>(
        &self,
        relative: &path::Path,
        visit: &mut F,
    ) -> io::Result<()> {
        for entry in self
            .workspace
            .walk(relative, |walkdir| walkdir.min_depth(1).max_depth(1))
        {
            let entry = entry?;
            let relative = entry.relative();
            let file_type = entry.file_type();

            match self.index.contains_key(relative) {
                true if file_type.is_dir() => self.walk(relative, visit)?,
                true => (),
                false if self.is_trackable(&entry)? => visit(entry),
                false => (),
            }
        }
        Ok(())
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

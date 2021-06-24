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
        self.walk(path::Path::new("."))
    }

    fn walk(&self, relative: &path::Path) -> io::Result<()> {
        for entry in self
            .workspace
            .walk(relative, |walkdir| walkdir.min_depth(1).max_depth(1))
        {
            let entry = entry?;
            let relative = entry.relative();
            let file_type = entry.file_type();

            match self.index.contains_key(relative) {
                true if file_type.is_dir() => self.walk(relative)?,
                true => self.visit_tracked(entry)?,
                false if self.is_trackable(&entry)? => self.visit_untracked(entry),
                false => (),
            }
        }
        Ok(())
    }

    fn visit_tracked(&self, entry: workspace::DirEntry) -> io::Result<()> {
        let tracked = self
            .index
            .get(entry.relative())
            .expect("[INTERNAL ERROR]: `Index::contains_key` inconsistent with `Index::get`");

        if tracked.metadata().size as u64 != entry.metadata()?.len() {
            println!(" M {}", entry.relative().display());
        }

        Ok(())
    }

    fn visit_untracked(&self, entry: workspace::DirEntry) {
        if entry.file_type().is_dir() {
            println!("?? {}/", entry.relative().display());
        } else {
            println!("?? {}", entry.relative().display());
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

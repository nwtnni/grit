use std::env;
use std::io;
use std::path;

use structopt::StructOpt;

use crate::workspace;

#[derive(StructOpt)]
pub struct Status {}

impl Status {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let git = root.join(".git");
        let workspace = crate::Workspace::new(root);
        let index = crate::Index::lock(&git)?;

        Self::walk(&workspace, &index, path::Path::new("."), &mut |entry| {
            if entry.file_type().is_dir() {
                println!("?? {}/", entry.relative().display());
            } else {
                println!("?? {}", entry.relative().display());
            }
        })?;

        Ok(())
    }

    fn walk<F: for<'a> FnMut(workspace::DirEntry<'a>)>(
        workspace: &crate::Workspace,
        index: &crate::Index,
        relative: &path::Path,
        visit: &mut F,
    ) -> io::Result<()> {
        for entry in workspace.walk(relative, |walkdir| walkdir.min_depth(1).max_depth(1)) {
            let entry = entry?;
            let relative = entry.relative();
            let file_type = entry.file_type();

            match index.contains_key(relative) {
                true if file_type.is_dir() => Self::walk(workspace, index, relative, visit)?,
                true => (),
                false if Self::is_trackable(workspace, index, &entry)? => visit(entry),
                false => (),
            }
        }
        Ok(())
    }

    fn is_trackable(
        workspace: &crate::Workspace,
        index: &crate::Index,
        entry: &workspace::DirEntry,
    ) -> io::Result<bool> {
        let relative = entry.relative();
        let file_type = entry.file_type();

        if file_type.is_file() {
            return Ok(!index.contains_key(relative));
        }

        if file_type.is_symlink() {
            unimplemented!();
        }

        for entry in workspace.walk(relative, |walkdir| {
            walkdir
                .min_depth(1)
                .max_depth(1)
                .sort_by_key(|entry| entry.file_type().is_dir())
        }) {
            if Self::is_trackable(workspace, index, &entry?)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

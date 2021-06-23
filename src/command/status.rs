use std::env;
use std::fs;
use std::io;
use std::path;

use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Status {}

impl Status {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let git = root.join(".git");
        let workspace = crate::Workspace::new(root);
        let index = crate::Index::lock(&git)?;

        Self::walk(&workspace, &index, path::Path::new("."), &mut |entry| {
            let relative = entry
                .path()
                .strip_prefix(workspace.root())
                .expect("[INTERNAL ERROR]: workspace must contain path");

            if entry.file_type().is_dir() {
                println!("?? {}/", relative.display());
            } else {
                println!("?? {}", relative.display());
            }
        })?;

        Ok(())
    }

    fn walk<F: FnMut(walkdir::DirEntry)>(
        workspace: &crate::Workspace,
        index: &crate::Index,
        relative: &path::Path,
        visit: &mut F,
    ) -> io::Result<()> {
        for entry in workspace.walk(relative, |walkdir| walkdir.min_depth(1).max_depth(1)) {
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(workspace.root())
                .expect("[INTERNAL ERROR]: workspace must contain path");
            if index.contains_key(relative) {
                if entry.file_type().is_dir() {
                    Self::walk(workspace, index, relative, visit)?;
                }
            } else if Self::is_trackable(workspace, index, relative, entry.file_type())? {
                visit(entry);
            }
        }
        Ok(())
    }

    fn is_trackable(
        workspace: &crate::Workspace,
        index: &crate::Index,
        relative: &path::Path,
        file_type: fs::FileType,
    ) -> io::Result<bool> {
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
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(workspace.root())
                .expect("[INTERNAL ERROR]: workspace must contain path");

            if Self::is_trackable(workspace, index, relative, entry.file_type())? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

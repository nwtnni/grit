use std::iter;
use std::os::unix::ffi::OsStrExt as _;
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

    pub fn walk<P: AsRef<path::Path>>(
        &self,
        relative: P,
    ) -> impl Iterator<Item = walkdir::Result<walkdir::DirEntry>> {
        let root = self.root.join(relative);
        let git = self.root.join(".git");
        walkdir::WalkDir::new(root)
            .contents_first(true)
            .sort_by(|prev, next| {
                let prev_name = prev.file_name().as_bytes();
                let next_name = next.file_name().as_bytes();

                // In order to match `git`'s sort order, we can to add a trailing
                // directory separator '/' to the end of directory names.
                //
                // This emulates sorting all files by their complete paths without
                // requiring us to store them all in memory.
                match (prev.file_type().is_dir(), next.file_type().is_dir()) {
                    (true, true) | (false, false) => prev_name.cmp(&next_name),
                    (true, false) => prev_name.iter().chain(iter::once(&b'/')).cmp(next_name),
                    (false, true) => prev_name
                        .iter()
                        .cmp(next_name.iter().chain(iter::once(&b'/'))),
                }
            })
            .into_iter()
            .filter(move |entry| match entry {
                Ok(entry) => !entry.path().starts_with(&git),
                Err(_) => true,
            })
    }
}

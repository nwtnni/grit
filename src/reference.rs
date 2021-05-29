use std::fs;
use std::io;
use std::io::Write as _;
use std::path;

use crate::file;
use crate::object;

#[derive(Clone, Debug)]
pub struct Reference {
    root: path::PathBuf,
    head: path::PathBuf,
}

impl Reference {
    pub fn new(git: &path::Path) -> Self {
        Reference {
            root: git.join("refs"),
            head: git.join("HEAD"),
        }
    }

    pub fn set_head(&self, id: &object::Id) -> io::Result<()> {
        let mut head = file::Lock::new(self.head.clone())?;
        write!(&mut *head, "{}", id)?;
        head.commit()
    }

    pub fn head(&self) -> io::Result<Option<object::Id>> {
        match fs::read(&self.head).and_then(|bytes| object::Id::from_bytes(&bytes)) {
            Ok(id) => Ok(Some(id)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

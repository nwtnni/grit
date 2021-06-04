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
        let mut head = file::WriteLock::new(self.head.clone())?;
        write!(&mut head, "{}", id)?;
        head.commit()
    }

    pub fn head(&self) -> io::Result<Option<object::Id>> {
        let mut head = match file::WriteLock::new(self.head.clone())?.upgrade()? {
            file::Lock::ReadWrite(lock) => lock,
            file::Lock::Write(_) => return Ok(None),
        };

        object::Id::read_hex(&mut head).map(Option::Some)
    }
}

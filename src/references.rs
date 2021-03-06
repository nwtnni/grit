use std::io;
use std::io::Write as _;
use std::path;

use crate::file;
use crate::object;

#[derive(Clone, Debug)]
pub struct References {
    root: path::PathBuf,
    head: path::PathBuf,
}

impl References {
    pub fn new(root: path::PathBuf, head: path::PathBuf) -> Self {
        References { root, head }
    }

    pub fn read_head(&self) -> anyhow::Result<Option<object::Id>> {
        let mut head = match file::WriteLock::new(self.head.clone())?.upgrade()? {
            file::Lock::ReadWrite(lock) => lock,
            file::Lock::Write(_) => return Ok(None),
        };

        object::Id::read_hex(&mut head).map(Option::Some)
    }

    pub fn write_head(&self, id: &object::Id) -> io::Result<()> {
        let mut head = file::WriteLock::new(self.head.clone())?;
        write!(&mut head, "{}", id)?;
        head.commit()
    }
}

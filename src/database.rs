use std::io;
use std::io::Write as _;
use std::path;

use crate::file;
use crate::object;
use crate::Object;

#[derive(Debug)]
pub struct Database {
    root: path::PathBuf,
}

impl Database {
    pub fn new(git: &path::Path) -> io::Result<Self> {
        Ok(Database {
            root: git.join("objects"),
        })
    }

    pub fn store(&self, object: &Object) -> io::Result<object::Id> {
        let data = object.encode();
        let id = object::Id::from(&data);
        let path = self.root.join(id.to_path_buf());

        let mut file = file::Temp::new(path)?;
        let mut stream =
            flate2::write::ZlibEncoder::new(&mut **file, flate2::Compression::default());

        stream.write_all(&data)?;
        stream.finish()?;
        file.commit()?;

        Ok(id)
    }
}

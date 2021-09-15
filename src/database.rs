use std::fs;
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
    pub fn new(root: path::PathBuf) -> Self {
        Database { root }
    }

    pub fn load(&self, id: &object::Id) -> anyhow::Result<Object> {
        let path = self.root.join(id.to_path_buf());

        let mut stream = fs::OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map(flate2::read::ZlibDecoder::new)
            .map(io::BufReader::new)?;

        Object::read(&mut stream)
    }

    pub fn store(&self, object: &Object) -> io::Result<object::Id> {
        let buffer = object.to_bytes();
        let id = object::Id::hash(&buffer);
        let path = self.root.join(id.to_path_buf());

        let mut file = match file::Temp::new(path) {
            Ok(file) => file,
            // Object has already been written to disk.
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(id),
            Err(error) => return Err(error),
        };

        let mut stream = flate2::write::ZlibEncoder::new(&mut file, flate2::Compression::default());

        stream.write_all(&buffer)?;
        stream.finish()?;
        file.commit()?;

        Ok(id)
    }
}

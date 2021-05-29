use std::fs;
use std::io;
use std::io::Write as _;
use std::path;

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

    pub fn store(&self, object: &Object) -> io::Result<()> {
        let data = object.encode();
        let id = object::Id::from(&*data);

        let mut path = self.root.join(id.directory());
        fs::create_dir(&path)?;
        path.push(id.file_name());

        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => flate2::write::ZlibEncoder::new(file, flate2::Compression::default()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(()),
            Err(error) => return Err(error),
        };

        // TODO: write to temp file and atomically rename
        file.write_all(&data)?;
        file.finish()?;
        Ok(())
    }
}

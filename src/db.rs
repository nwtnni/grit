use std::fmt;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path;

use sha1::Sha1;

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
        let id = ObjectId::from(&*data);

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

#[derive(Clone, Debug)]
pub enum Object {
    Blob(Vec<u8>),
}

impl Object {
    fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        buffer.extend_from_slice(self.r#type());
        buffer.push(b' ');
        write!(&mut buffer, "{}", self.len()).expect("[UNREACHABLE]: write to `Vec` failed");
        buffer.push(0);

        match self {
            Object::Blob(blob) => buffer.extend_from_slice(&blob),
        }

        buffer
    }

    fn r#type(&self) -> &'static [u8] {
        match self {
            Object::Blob(_) => b"blob",
        }
    }

    fn len(&self) -> usize {
        match self {
            Object::Blob(blob) => blob.len(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 20]);

impl ObjectId {
    #[inline]
    fn directory(&self) -> path::PathBuf {
        path::PathBuf::from(format!("{:02x}", self.0[0]))
    }

    #[inline]
    fn file_name(&self) -> path::PathBuf {
        let mut buffer = String::new();
        for byte in &self.0[1..] {
            write!(&mut buffer, "{:02x}", byte).expect("[UNREACHABLE]: write to `String` failed");
        }
        path::PathBuf::from(buffer)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.0 {
            write!(fmt, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl<T: AsRef<[u8]>> From<T> for ObjectId {
    fn from(data: T) -> Self {
        ObjectId(Sha1::from(data.as_ref()).digest().bytes())
    }
}

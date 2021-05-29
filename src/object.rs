use std::fmt;
use std::fmt::Write as _;
use std::path;

use sha1::Sha1;

mod blob;

pub use blob::Blob;

#[derive(Clone, Debug)]
pub enum Object {
    Blob(Blob),
}

impl Object {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Object::Blob(blob) => blob.encode(),
        }
    }

    pub fn r#type(&self) -> &'static [u8] {
        match self {
            Object::Blob(blob) => blob.r#type(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Object::Blob(blob) => blob.len(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id([u8; 20]);

impl Id {
    #[inline]
    pub fn directory(&self) -> path::PathBuf {
        path::PathBuf::from(format!("{:02x}", self.0[0]))
    }

    #[inline]
    pub fn file_name(&self) -> path::PathBuf {
        let mut buffer = String::new();
        for byte in &self.0[1..] {
            write!(&mut buffer, "{:02x}", byte).expect("[UNREACHABLE]: write to `String` failed");
        }
        path::PathBuf::from(buffer)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.0 {
            write!(fmt, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl<T: AsRef<[u8]>> From<T> for Id {
    fn from(data: T) -> Self {
        Self(Sha1::from(data.as_ref()).digest().bytes())
    }
}

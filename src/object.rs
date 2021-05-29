use std::fmt;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path;

use sha1::Sha1;

pub mod blob;
pub mod commit;
pub mod tree;

pub use blob::Blob;
pub use commit::Commit;
pub use tree::Tree;

#[derive(Clone, Debug)]
pub enum Object {
    Blob(Blob),
    Commit(Commit),
    Tree(Tree),
}

impl Object {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();

        buffer.extend_from_slice(self.r#type().as_bytes());
        buffer.push(b' ');

        write!(&mut buffer, "{}", self.len()).expect("[UNREACHABLE]: write to `Vec` failed");
        buffer.push(0);

        match self {
            Object::Blob(blob) => blob.encode_mut(&mut buffer),
            Object::Commit(commit) => commit.encode_mut(&mut buffer),
            Object::Tree(tree) => tree.encode_mut(&mut buffer),
        }

        buffer
    }

    fn r#type(&self) -> &'static str {
        match self {
            Object::Blob(blob) => blob.r#type(),
            Object::Commit(commit) => commit.r#type(),
            Object::Tree(tree) => tree.r#type(),
        }
    }

    fn len(&self) -> usize {
        match self {
            Object::Blob(blob) => blob.len(),
            Object::Commit(commit) => commit.len(),
            Object::Tree(tree) => tree.len(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id([u8; 20]);

impl Id {
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn directory(&self) -> path::PathBuf {
        path::PathBuf::from(format!("{:02x}", self.0[0]))
    }

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

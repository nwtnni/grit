#![allow(clippy::len_without_is_empty)]

use std::fmt;
use std::io;
use std::path;
use std::str;

use sha1::Sha1;

use crate::util::hex;
use crate::util::Tap as _;

mod blob;
mod commit;
mod person;
mod tree;

pub use blob::Blob;
pub use commit::Commit;
pub use person::Person;
pub use tree::Tree;
pub use tree::TreeNode;

#[derive(Clone, Debug)]
pub enum Object {
    Blob(Blob),
    Commit(Commit),
    Tree(Tree),
}

impl Object {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut cursor = io::Cursor::new(&mut buffer);
        self.write(&mut cursor)
            .expect("[INTERNAL ERROR]: write to `Vec` failed");
        buffer
    }

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        let mut r#type = Vec::new();
        reader.read_until(b' ', &mut r#type)?;
        assert_eq!(r#type.pop(), Some(b' '));

        // TODO: validate length when parsing
        let mut len = Vec::new();
        reader.read_until(0, &mut len)?;
        assert_eq!(len.pop(), Some(0));
        let _len = String::from_utf8(len).unwrap().parse::<usize>().unwrap();

        match &*r#type {
            Blob::TYPE => Blob::read(reader).map(Object::Blob),
            Commit::TYPE => Commit::read(reader).map(Object::Commit),
            Tree::TYPE => Tree::read(reader).map(Object::Tree),
            _ => unreachable!(),
        }
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.r#type())?;
        writer.write_all(b" ")?;

        write!(writer, "{}\0", self.len())?;

        match self {
            Object::Blob(blob) => blob.write(writer),
            Object::Commit(commit) => commit.write(writer),
            Object::Tree(tree) => tree.write(writer),
        }
    }

    fn r#type(&self) -> &'static [u8] {
        match self {
            Object::Blob(_) => Blob::TYPE,
            Object::Commit(_) => Commit::TYPE,
            Object::Tree(_) => Tree::TYPE,
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
    pub fn hash(bytes: &[u8]) -> Self {
        Self(Sha1::from(bytes).digest().bytes())
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn read_bytes<R: io::Read>(reader: &mut R) -> anyhow::Result<Self> {
        let mut buffer = [0u8; 20];
        reader.read_exact(&mut buffer)?;
        Ok(Self(buffer))
    }

    pub fn read_hex<R: io::Read>(reader: &mut R) -> anyhow::Result<Self> {
        let mut buffer = [0u8; 40];
        reader.read_exact(&mut buffer)?;

        let mut id = [0u8; 20];

        buffer
            .chunks(2)
            .zip(&mut id)
            .for_each(|(source, target)| *target = hex::decode(source[0], source[1]));

        Ok(Id(id))
    }

    pub fn to_path_buf(self) -> path::PathBuf {
        let mut buffer = String::with_capacity(40);
        let [hi, lo] = hex::encode(self.0[0]);
        buffer.push(hi as char);
        buffer.push(lo as char);
        buffer.push('/');
        for byte in &self.0[1..] {
            let [hi, lo] = hex::encode(*byte);
            buffer.push(hi as char);
            buffer.push(lo as char);
        }
        path::PathBuf::from(buffer)
    }

    pub fn write_bytes<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    pub fn write_hex<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.0
            .iter()
            .copied()
            .map(hex::encode)
            .try_for_each(|code| writer.write_all(&code))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.0 {
            let [hi, lo] = hex::encode(*byte);
            write!(fmt, "{}{}", hi as char, lo as char)?;
        }
        Ok(())
    }
}

impl str::FromStr for Id {
    type Err = anyhow::Error;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        string
            .as_bytes()
            .tap(io::Cursor::new)
            .tap(|mut cursor| Id::read_hex(&mut cursor))
    }
}

use std::io;
use std::str;

use byteorder::ReadBytesExt as _;

use crate::object;
use crate::object::Author;

#[derive(Clone, Debug)]
pub struct Commit {
    tree: object::Id,
    parent: Option<object::Id>,
    author: Author,
    message: String,
}

impl Commit {
    pub const TYPE: &'static [u8] = b"commit";

    pub fn new(
        tree: object::Id,
        parent: Option<object::Id>,
        author: Author,
        message: String,
    ) -> Self {
        Commit {
            tree,
            parent,
            author,
            message,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn tree(&self) -> &object::Id {
        &self.tree
    }

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        let mut tag = Vec::new();
        reader.read_until(b' ', &mut tag)?;
        assert_eq!(tag, b"tree ");

        let tree = object::Id::read_hex(reader)?;
        assert_eq!(reader.read_u8()?, b'\n');

        tag.clear();
        reader.read_until(b' ', &mut tag)?;

        let parent = if tag == b"parent " {
            let parent = object::Id::read_hex(reader)?;
            assert_eq!(reader.read_u8()?, b'\n');
            tag.clear();
            reader.read_until(b' ', &mut tag)?;
            Some(parent)
        } else {
            None
        };

        assert_eq!(tag, b"author ");
        let author = Author::read(reader)?;
        assert_eq!(reader.read_u8()?, b'\n');

        tag.clear();
        reader.read_until(b' ', &mut tag)?;

        // TODO: store committer separately
        assert_eq!(tag, b"committer ");
        let _committer = Author::read(reader)?;
        assert_eq!(reader.read_u8()?, b'\n');
        assert_eq!(reader.read_u8()?, b'\n');

        let mut message = String::new();
        reader.read_to_string(&mut message)?;
        Ok(Commit {
            tree,
            parent,
            author,
            message,
        })
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(b"tree ")?;
        self.tree.write_hex(writer)?;

        if let Some(parent) = self.parent {
            writer.write_all(b"\nparent ")?;
            parent.write_hex(writer)?;
        }

        writer.write_all(b"\nauthor ")?;
        self.author.write(writer)?;

        writer.write_all(b"\ncommitter ")?;
        self.author.write(writer)?;

        writer.write_all(b"\n\n")?;
        writer.write_all(self.message.as_bytes())
    }

    pub fn len(&self) -> usize {
        5 + self.tree.as_bytes().len() * 2
            + if let Some(parent) = self.parent {
                8 + parent.as_bytes().len() * 2
            } else {
                0
            }
            + 8
            + self.author.len()
            + 11
            + self.author.len()
            + 2
            + self.message.len()
    }
}

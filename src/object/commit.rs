use std::io;
use std::io::Write as _;

use crate::object;

#[derive(Clone, Debug)]
pub struct Commit {
    tree: object::Id,
    parent: Option<object::Id>,
    author: Author,
    message: String,
}

impl Commit {
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

    pub fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(b"tree ")?;
        self.tree.write(&mut writer)?;

        if let Some(parent) = self.parent {
            writer.write_all(b"\nparent ")?;
            parent.write(&mut writer)?;
        }

        writer.write_all(b"\nauthor ")?;
        self.author.write(&mut writer)?;

        writer.write_all(b"\ncommitter ")?;
        self.author.write(&mut writer)?;

        writer.write_all(b"\n\n")?;
        writer.write_all(self.message.as_bytes())
    }

    pub fn r#type(&self) -> &'static str {
        "commit"
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

#[derive(Clone, Debug)]
pub struct Author {
    name: String,
    email: String,
    time: chrono::DateTime<chrono::Local>,
}

impl Author {
    pub fn new(name: String, email: String, time: chrono::DateTime<chrono::Local>) -> Self {
        Author { name, email, time }
    }

    fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(self.name.as_bytes())?;
        writer.write_all(b" <")?;
        writer.write_all(self.email.as_bytes())?;
        writer.write_all(b"> ")?;
        write!(writer, "{}", self.time.format("%s %z"))
    }

    fn len(&self) -> usize {
        let mut buffer = [0u8; 16];
        let mut cursor = io::Cursor::new(&mut buffer[..]);

        // TODO: is it possible to calculate the length without writing?
        write!(&mut cursor, "{}", self.time.format("%s"))
            .expect("[UNREACHABLE]: Unix timestamp larger than 16 bytes");

        self.name.len() + 2 + self.email.len() + 2 + cursor.position() as usize + 1 + 5
    }
}

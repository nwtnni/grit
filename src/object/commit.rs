use std::io;
use std::io::Write as _;
use std::str;

use byteorder::ReadBytesExt as _;

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

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        let tree = object::Id::read_hex(reader)?;
        assert_eq!(reader.read_u8()?, b'\n');

        let mut tag = Vec::new();
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

    fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        let mut name = Vec::new();
        reader.read_until(b'<', &mut name)?;
        assert_eq!(name.pop(), Some(b'>'));
        assert_eq!(name.pop(), Some(b' '));
        let name = String::from_utf8(name)?;

        let mut email = Vec::new();
        reader.read_until(b' ', &mut email)?;
        assert_eq!(email.pop(), Some(b' '));
        assert_eq!(email.pop(), Some(b'>'));
        let email = String::from_utf8(email)?;

        let mut time = Vec::new();
        reader.read_until(b' ', &mut time)?;
        let lo = time.len();
        time.extend([0; 5]);
        reader.read_exact(&mut time[lo..])?;
        let time = str::from_utf8(&time)
            .map(|time| chrono::DateTime::parse_from_str(time, "%s %z"))?
            .map(|time| time.with_timezone(&chrono::Local))?;

        Ok(Self { name, email, time })
    }

    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
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

use std::io;
use std::io::Write as _;
use std::str;

#[derive(Clone, Debug)]
pub struct Person {
    name: String,
    email: String,
    time: chrono::DateTime<chrono::Local>,
}

impl Person {
    pub fn new(name: String, email: String, time: chrono::DateTime<chrono::Local>) -> Self {
        Person { name, email, time }
    }

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        let mut name = Vec::new();
        reader.read_until(b'<', &mut name)?;
        assert_eq!(name.pop(), Some(b'<'));
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

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.name.as_bytes())?;
        writer.write_all(b" <")?;
        writer.write_all(self.email.as_bytes())?;
        writer.write_all(b"> ")?;
        write!(writer, "{}", self.time.format("%s %z"))
    }

    pub fn len(&self) -> usize {
        let mut buffer = [0u8; 16];
        let mut cursor = io::Cursor::new(&mut buffer[..]);

        // TODO: is it possible to calculate the length without writing?
        write!(&mut cursor, "{}", self.time.format("%s"))
            .expect("[UNREACHABLE]: Unix timestamp larger than 16 bytes");

        self.name.len() + 2 + self.email.len() + 2 + cursor.position() as usize + 1 + 5
    }
}

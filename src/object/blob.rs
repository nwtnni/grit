use std::io;

#[derive(Clone, Debug)]
pub struct Blob(Vec<u8>);

impl Blob {
    pub const TYPE: &'static [u8] = b"blob";

    pub fn new(data: Vec<u8>) -> Self {
        Blob(data)
    }

    pub fn read<R: io::Read>(reader: &mut R) -> anyhow::Result<Self> {
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(Self(buffer))
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

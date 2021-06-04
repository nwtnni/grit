use std::io;

#[derive(Clone, Debug)]
pub struct Blob(Vec<u8>);

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Blob(data)
    }

    pub fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(&self.0)
    }

    pub fn r#type(&self) -> &'static str {
        "blob"
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

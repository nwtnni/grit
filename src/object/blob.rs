use std::io::Write as _;

#[derive(Clone, Debug)]
pub struct Blob(Vec<u8>);

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Blob(data)
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(self.r#type());
        buffer.push(b' ');
        write!(&mut buffer, "{}", self.len()).expect("[UNREACHABLE]: write to `Vec` failed");
        buffer.push(0);
        buffer.extend_from_slice(&self.0);
        buffer
    }

    pub fn r#type(&self) -> &'static [u8] {
        b"blob"
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

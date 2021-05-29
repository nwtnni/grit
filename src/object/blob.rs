#[derive(Clone, Debug)]
pub struct Blob(Vec<u8>);

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Blob(data)
    }

    pub fn encode_mut(&self, buffer: &mut Vec<u8>) {
        buffer.extend_from_slice(&self.0);
    }

    pub fn r#type(&self) -> &'static str {
        "blob"
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

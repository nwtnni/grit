use std::fmt;
use std::io;
use std::io::Write as _;
use std::path;
use std::str;

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

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        // Strip trailing newline
        let bytes = bytes.strip_suffix(&[b'\n']).unwrap_or(bytes);

        if bytes.len() != 40 || bytes.iter().any(|byte| HEX_DECODE[*byte as usize] == 255) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected 40 hexadecimal characters, but found {}: {:02x?}",
                    bytes.len(),
                    bytes,
                ),
            ));
        }

        let mut id = [0u8; 20];

        for (source, target) in bytes.chunks(2).zip(&mut id) {
            *target = hex_decode(source);
        }

        Ok(Id(id))
    }

    pub fn directory(&self) -> path::PathBuf {
        let mut buffer = String::with_capacity(2);
        let (hi, lo) = hex_encode(self.0[0]);
        buffer.push(hi as char);
        buffer.push(lo as char);
        path::PathBuf::from(buffer)
    }

    pub fn file_name(&self) -> path::PathBuf {
        let mut buffer = String::with_capacity(38);
        for byte in &self.0[1..] {
            let (hi, lo) = hex_encode(*byte);
            buffer.push(hi as char);
            buffer.push(lo as char);
        }
        path::PathBuf::from(buffer)
    }

    pub fn encode_mut(&self, buffer: &mut Vec<u8>) {
        for byte in &self.0 {
            let (hi, lo) = hex_encode(*byte);
            buffer.push(hi as u8);
            buffer.push(lo as u8);
        }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        for byte in &self.0 {
            let (hi, lo) = hex_encode(*byte);
            write!(fmt, "{}{}", hi as char, lo as char)?;
        }
        Ok(())
    }
}

impl<T: AsRef<[u8]>> From<T> for Id {
    fn from(data: T) -> Self {
        Self(Sha1::from(data.as_ref()).digest().bytes())
    }
}

fn hex_encode(byte: u8) -> (u8, u8) {
    let hi = byte >> 4;
    let lo = byte & 0b1111;
    (HEX_ENCODE[hi as usize], HEX_ENCODE[lo as usize])
}

fn hex_decode(code: &[u8]) -> u8 {
    let hi = HEX_DECODE[code[0] as usize];
    let lo = HEX_DECODE[code[1] as usize];
    hi << 4 | lo
}

static HEX_ENCODE: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

static HEX_DECODE: [u8; 256] = [
    255, // 0x00
    255, // 0x01
    255, // 0x02
    255, // 0x03
    255, // 0x04
    255, // 0x05
    255, // 0x06
    255, // 0x07
    255, // 0x08
    255, // 0x09
    255, // 0x0a
    255, // 0x0b
    255, // 0x0c
    255, // 0x0d
    255, // 0x0e
    255, // 0x0f
    255, // 0x10
    255, // 0x11
    255, // 0x12
    255, // 0x13
    255, // 0x14
    255, // 0x15
    255, // 0x16
    255, // 0x17
    255, // 0x18
    255, // 0x19
    255, // 0x1a
    255, // 0x1b
    255, // 0x1c
    255, // 0x1d
    255, // 0x1e
    255, // 0x1f
    255, // 0x20
    255, // 0x21
    255, // 0x22
    255, // 0x23
    255, // 0x24
    255, // 0x25
    255, // 0x26
    255, // 0x27
    255, // 0x28
    255, // 0x29
    255, // 0x2a
    255, // 0x2b
    255, // 0x2c
    255, // 0x2d
    255, // 0x2e
    255, // 0x2f
    000, // 0x30 (0)
    001, // 0x31 (1)
    002, // 0x32 (2)
    003, // 0x33 (3)
    004, // 0x34 (4)
    005, // 0x35 (5)
    006, // 0x36 (6)
    007, // 0x37 (7)
    008, // 0x38 (8)
    009, // 0x39 (9)
    255, // 0x3a
    255, // 0x3b
    255, // 0x3c
    255, // 0x3d
    255, // 0x3e
    255, // 0x3f
    255, // 0x40
    010, // 0x41 (A)
    011, // 0x42 (B)
    012, // 0x43 (C)
    013, // 0x44 (D)
    014, // 0x45 (E)
    015, // 0x46 (F)
    255, // 0x47
    255, // 0x48
    255, // 0x49
    255, // 0x4a
    255, // 0x4b
    255, // 0x4c
    255, // 0x4d
    255, // 0x4e
    255, // 0x4f
    255, // 0x50
    255, // 0x51
    255, // 0x52
    255, // 0x53
    255, // 0x54
    255, // 0x55
    255, // 0x56
    255, // 0x57
    255, // 0x58
    255, // 0x59
    255, // 0x5a
    255, // 0x5b
    255, // 0x5c
    255, // 0x5d
    255, // 0x5e
    255, // 0x5f
    255, // 0x60
    010, // 0x61 (a)
    011, // 0x62 (b)
    012, // 0x63 (c)
    013, // 0x64 (d)
    014, // 0x65 (e)
    015, // 0x66 (f)
    255, // 0x67
    255, // 0x68
    255, // 0x69
    255, // 0x6a
    255, // 0x6b
    255, // 0x6c
    255, // 0x6d
    255, // 0x6e
    255, // 0x6f
    255, // 0x70
    255, // 0x71
    255, // 0x72
    255, // 0x73
    255, // 0x74
    255, // 0x75
    255, // 0x76
    255, // 0x77
    255, // 0x78
    255, // 0x79
    255, // 0x7a
    255, // 0x7b
    255, // 0x7c
    255, // 0x7d
    255, // 0x7e
    255, // 0x7f
    255, // 0x80
    255, // 0x81
    255, // 0x82
    255, // 0x83
    255, // 0x84
    255, // 0x85
    255, // 0x86
    255, // 0x87
    255, // 0x88
    255, // 0x89
    255, // 0x8a
    255, // 0x8b
    255, // 0x8c
    255, // 0x8d
    255, // 0x8e
    255, // 0x8f
    255, // 0x90
    255, // 0x91
    255, // 0x92
    255, // 0x93
    255, // 0x94
    255, // 0x95
    255, // 0x96
    255, // 0x97
    255, // 0x98
    255, // 0x99
    255, // 0x9a
    255, // 0x9b
    255, // 0x9c
    255, // 0x9d
    255, // 0x9e
    255, // 0x9f
    255, // 0xa0
    255, // 0xa1
    255, // 0xa2
    255, // 0xa3
    255, // 0xa4
    255, // 0xa5
    255, // 0xa6
    255, // 0xa7
    255, // 0xa8
    255, // 0xa9
    255, // 0xaa
    255, // 0xab
    255, // 0xac
    255, // 0xad
    255, // 0xae
    255, // 0xaf
    255, // 0xb0
    255, // 0xb1
    255, // 0xb2
    255, // 0xb3
    255, // 0xb4
    255, // 0xb5
    255, // 0xb6
    255, // 0xb7
    255, // 0xb8
    255, // 0xb9
    255, // 0xba
    255, // 0xbb
    255, // 0xbc
    255, // 0xbd
    255, // 0xbe
    255, // 0xbf
    255, // 0xc0
    255, // 0xc1
    255, // 0xc2
    255, // 0xc3
    255, // 0xc4
    255, // 0xc5
    255, // 0xc6
    255, // 0xc7
    255, // 0xc8
    255, // 0xc9
    255, // 0xca
    255, // 0xcb
    255, // 0xcc
    255, // 0xcd
    255, // 0xce
    255, // 0xcf
    255, // 0xd0
    255, // 0xd1
    255, // 0xd2
    255, // 0xd3
    255, // 0xd4
    255, // 0xd5
    255, // 0xd6
    255, // 0xd7
    255, // 0xd8
    255, // 0xd9
    255, // 0xda
    255, // 0xdb
    255, // 0xdc
    255, // 0xdd
    255, // 0xde
    255, // 0xdf
    255, // 0xe0
    255, // 0xe1
    255, // 0xe2
    255, // 0xe3
    255, // 0xe4
    255, // 0xe5
    255, // 0xe6
    255, // 0xe7
    255, // 0xe8
    255, // 0xe9
    255, // 0xea
    255, // 0xeb
    255, // 0xec
    255, // 0xed
    255, // 0xee
    255, // 0xef
    255, // 0xf0
    255, // 0xf1
    255, // 0xf2
    255, // 0xf3
    255, // 0xf4
    255, // 0xf5
    255, // 0xf6
    255, // 0xf7
    255, // 0xf8
    255, // 0xf9
    255, // 0xfa
    255, // 0xfb
    255, // 0xfc
    255, // 0xfd
    255, // 0xfe
    255, // 0xff
];

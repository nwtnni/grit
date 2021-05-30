use std::cmp;
use std::fs;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path;

use crate::object;

// Invariant: sorted
#[derive(Clone, Debug)]
pub struct Tree(Vec<Node>);

impl Tree {
    pub fn new(nodes: Vec<Node>) -> Self {
        Tree(nodes)
    }

    pub fn encode_mut(&self, buffer: &mut Vec<u8>) {
        for node in &self.0 {
            node.encode_mut(buffer);
        }
    }

    pub fn r#type(&self) -> &'static str {
        "tree"
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Node::len).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    path: path::PathBuf,
    id: object::Id,
    mode: Mode,
}

impl Node {
    pub fn new(path: path::PathBuf, id: object::Id, meta: fs::Metadata) -> Self {
        Node {
            path,
            id,
            mode: Mode::from(meta),
        }
    }

    pub fn id(&self) -> &object::Id {
        &self.id
    }

    fn encode_mut(&self, buffer: &mut Vec<u8>) {
        self.mode.encode_mut(buffer);
        buffer.push(b' ');
        buffer.extend_from_slice(self.path.as_os_str().as_bytes());
        buffer.push(0);
        buffer.extend_from_slice(self.id.as_bytes());
    }

    fn len(&self) -> usize {
        self.mode.len() + 1 + self.path.as_os_str().as_bytes().len() + 1 + self.id.as_bytes().len()
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.path
            .cmp(&other.path)
            .then_with(|| self.id.cmp(&other.id))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Mode {
    Directory,
    Regular,
    Executable,
}

impl Mode {
    fn encode_mut(&self, buffer: &mut Vec<u8>) {
        match self {
            Mode::Directory => buffer.extend_from_slice(b"40000"),
            Mode::Regular => buffer.extend_from_slice(b"100644"),
            Mode::Executable => buffer.extend_from_slice(b"100755"),
        }
    }

    fn len(&self) -> usize {
        match self {
            Mode::Directory => 5,
            Mode::Regular => 6,
            Mode::Executable => 6,
        }
    }
}

impl From<fs::Metadata> for Mode {
    fn from(meta: fs::Metadata) -> Self {
        if meta.file_type().is_dir() {
            Mode::Directory
        } else if meta.permissions().mode() & 0o111 > 0 {
            Mode::Executable
        } else {
            Mode::Regular
        }
    }
}

use std::cmp;
use std::convert::TryFrom as _;
use std::ffi;
use std::io;
use std::iter;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::ffi::OsStringExt as _;
use std::path;
use std::slice;
use std::vec;

use crate::meta;
use crate::object;
use crate::util::Tap as _;

// Invariant: sorted
#[derive(Clone, Debug)]
pub struct Root(Vec<Node>);

impl Root {
    pub const TYPE: &'static [u8] = b"tree";

    pub fn new(nodes: Vec<Node>) -> Self {
        Root(nodes)
    }

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Self> {
        iter::from_fn(|| Node::read(reader).transpose())
            .collect::<Result<Vec<_>, _>>()
            .map(Root)
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.0.iter().try_for_each(|node| node.write(writer))
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(Node::len).sum()
    }
}

impl IntoIterator for Root {
    type IntoIter = vec::IntoIter<Node>;
    type Item = Node;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Root {
    type IntoIter = slice::Iter<'a, Node>;
    type Item = &'a Node;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    path: path::PathBuf,
    id: object::Id,
    mode: meta::Mode,
}

impl Node {
    pub fn new(path: path::PathBuf, id: object::Id, mode: meta::Mode) -> Self {
        Node { path, id, mode }
    }

    pub fn id(&self) -> &object::Id {
        &self.id
    }

    pub fn mode(&self) -> &meta::Mode {
        &self.mode
    }

    pub fn path(&self) -> &path::Path {
        &self.path
    }

    pub fn read<R: io::BufRead>(reader: &mut R) -> anyhow::Result<Option<Self>> {
        let mut mode = Vec::new();
        reader.read_until(b' ', &mut mode)?;
        match mode.pop() {
            None => return Ok(None),
            Some(byte) => assert_eq!(byte, b' '),
        }
        // TODO: error handling?
        let mode = String::from_utf8(mode)
            .unwrap()
            .tap(|mode| meta::Mode::try_from(&*mode))
            .unwrap();

        let mut path = Vec::new();
        reader.read_until(0, &mut path)?;
        assert_eq!(path.pop(), Some(0));
        let path = ffi::OsString::from_vec(path).tap(path::PathBuf::from);

        let id = object::Id::read_bytes(reader)?;
        Ok(Some(Self { path, id, mode }))
    }

    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.mode.as_str().as_bytes())?;
        writer.write_all(b" ")?;
        writer.write_all(self.path.as_os_str().as_bytes())?;
        writer.write_all(b"\0")?;
        writer.write_all(self.id.as_bytes())
    }

    fn len(&self) -> usize {
        self.mode.as_str().len()
            + 1
            + self.path.as_os_str().as_bytes().len()
            + 1
            + self.id.as_bytes().len()
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

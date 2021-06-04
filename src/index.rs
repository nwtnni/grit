use std::cmp;
use std::collections::btree_set;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::convert::TryFrom as _;
use std::ffi;
use std::fs;
use std::io;
use std::io::Read as _;
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::ffi::OsStringExt as _;
use std::path;

use byteorder::BigEndian;
use byteorder::ReadBytesExt as _;
use byteorder::WriteBytesExt as _;

use crate::file;
use crate::meta;
use crate::object;
use crate::util::Tap as _;

pub struct Index {
    lock: file::Checksum<file::WriteLock>,
    entries: BTreeSet<Entry>,
    changed: bool,
}

impl Index {
    pub fn lock(git: &path::Path) -> io::Result<Self> {
        let path = git.join("index");
        let lock = file::WriteLock::new(path)?.upgrade()?;

        let (entries, lock) = match lock {
            file::Lock::Write(lock) => (BTreeSet::new(), file::Checksum::new(lock)),
            file::Lock::ReadWrite(lock) => {
                let mut lock = file::Checksum::new(lock);
                let entries = Self::read(&mut lock)?;
                let lock = lock
                    .verify_checksum()?
                    .tap(file::ReadWriteLock::downgrade)
                    .tap(file::Checksum::new);
                (entries, lock)
            }
        };

        Ok(Index {
            lock,
            entries,
            changed: false,
        })
    }

    fn read<R: io::Read>(reader: &mut R) -> io::Result<BTreeSet<Entry>> {
        let mut header = [0u8; 4];
        reader.read_exact(&mut header)?;
        if &header != b"DIRC" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected `DIRC` signature bytes, but found {:?}", header),
            ));
        }

        let version = reader.read_u32::<BigEndian>()?;
        if version != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Expected version 2, but found {}", version),
            ));
        }

        let count = match reader.read_u32::<BigEndian>()?.tap(usize::try_from) {
            Ok(count) => count,
            Err(error) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Entry count does not fit in u32: {}", error),
                ))
            }
        };

        let mut entries = BTreeSet::new();
        for _ in 0..count {
            entries.insert(Entry::read(reader)?);
        }

        Ok(entries)
    }

    pub fn insert(&mut self, meta: &fs::Metadata, id: object::Id, path: path::PathBuf) {
        self.changed |= self.entries.insert(Entry::new(meta, id, path));
    }

    pub fn commit(mut self) -> io::Result<()> {
        if !self.changed {
            return Ok(());
        }

        let len = self
            .entries
            .len()
            .tap(u32::try_from)
            .expect("[INTERNAL ERROR]: more than 2^32 - 1 entries");

        self.lock.write_all(b"DIRC")?;
        self.lock.write_u32::<BigEndian>(2)?;
        self.lock.write_u32::<BigEndian>(len)?;
        for entry in &self.entries {
            entry.write(&mut self.lock)?;
        }
        self.lock.write_checksum()?.commit()
    }
}

impl<'a> IntoIterator for &'a Index {
    type IntoIter = Iter<'a>;
    type Item = Node<'a>;
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(&self.entries)
    }
}

/// Iterator over both files and directories represented in the index, in sorted
/// order. Directory contents will be yielded before the directory itself.
#[derive(Debug)]
pub struct Iter<'a> {
    iter: btree_set::Iter<'a, Entry>,
    state: Option<State<'a>>,
    queue: VecDeque<&'a path::Path>,
}

impl<'a> Iter<'a> {
    fn new(entries: &'a BTreeSet<Entry>) -> Self {
        let mut iter = entries.iter();
        let state = iter.next().map(State::Yield);
        Iter {
            iter,
            state,
            queue: VecDeque::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum State<'a> {
    Yield(&'a Entry),
    Yielded(&'a Entry),
}

#[derive(Copy, Clone, Debug)]
pub enum Node<'a> {
    File(&'a Entry),
    Directory(&'a path::Path),
}

impl<'a> Node<'a> {
    pub fn path(&self) -> &'a path::Path {
        match self {
            Node::File(entry) => entry.path(),
            Node::Directory(path) => path,
        }
    }

    pub fn mode(&self) -> &meta::Mode {
        match self {
            Node::File(entry) => entry.metadata().mode(),
            Node::Directory(_) => &meta::Mode::Directory,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Node<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        // First, yield any available directories.
        if let Some(directory) = self.queue.pop_front() {
            return Some(Node::Directory(directory));
        }

        // Otherwise, if there is a file that has not been yielded, then yield it.
        let prev = match self.state? {
            State::Yielded(prev) => prev,
            State::Yield(prev) => {
                self.state = Some(State::Yielded(prev));
                return Some(Node::File(prev));
            }
        };

        // Finally, compare the previous and next file paths to determine what
        // directories need to be yielded in between. Store these directories
        // and the next file to yield.
        let next = self.iter.next();

        // Yield any ancestor directories that differ between the previous and next.
        //
        // If this is the last file remaining, then yield all of its ancestors,
        // including the root ("").
        //
        // Examples:
        //
        // ```
        // a / b / c / 1.txt
        //   |
        // a / d / c / e / 2.txt
        //
        // Yields:
        //
        // a / b / c
        // a / b
        //
        // ---
        //
        // a / d / c / e / 2.txt
        // a / d / c / e / f / 3.txt
        //
        // Yields nothing.
        //
        // ---
        //
        // a / d / c / e / f / 3.txt
        // 4.txt
        //
        // Yields:
        //
        // a / d / c / e / f
        // a / d / c / e
        // a / d / c
        // a / d
        // a
        // ```
        prev.path
            .ancestors()
            .skip(1)
            .take_while(|ancestor| next.map_or(true, |next| !next.path.starts_with(ancestor)))
            .for_each(|ancestor| self.queue.push_back(ancestor));

        self.state = next.map(State::Yield);
        self.next()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    meta: meta::Data,
    id: object::Id,
    flag: u16,
    path: path::PathBuf,
}

impl Entry {
    pub fn new(meta: &fs::Metadata, id: object::Id, path: path::PathBuf) -> Self {
        let meta =
            meta::Data::try_from(meta).expect("[INTERNAL ERROR]: failed to convert metadata");
        let flag = cmp::min(0xFFF, path.as_os_str().as_bytes().len()) as u16;
        Entry {
            meta,
            id,
            flag,
            path,
        }
    }

    pub fn metadata(&self) -> &meta::Data {
        &self.meta
    }

    pub fn id(&self) -> &object::Id {
        &self.id
    }

    pub fn path(&self) -> &path::Path {
        &self.path
    }

    fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let meta = meta::Data::read(reader)?;
        let id = object::Id::read_bytes(reader)?;
        let flag = reader.read_u16::<BigEndian>()?;

        let mut buffer = Vec::new();
        reader.by_ref().take(2).read_to_end(&mut buffer)?;

        while !buffer.ends_with(&[0]) {
            reader.by_ref().take(8).read_to_end(&mut buffer)?;
        }

        while buffer.ends_with(&[0]) {
            buffer.pop();
        }

        Ok(Self {
            meta,
            id,
            flag,
            path: buffer.tap(ffi::OsString::from_vec).tap(path::PathBuf::from),
        })
    }

    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        self.meta.write(writer)?;
        writer.write_all(self.id.as_bytes())?;
        writer.write_u16::<BigEndian>(self.flag)?;
        writer.write_all(self.path.as_os_str().as_bytes())?;
        for _ in 0..self.padding() {
            writer.write_u8(0)?;
        }

        Ok(())
    }

    fn len(&self) -> usize {
        self.meta.len() + self.id.as_bytes().len() + 2 + self.path.as_os_str().as_bytes().len()
    }

    fn padding(&self) -> usize {
        0b1000 - (self.len() & 0b0111)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.path
            .as_os_str()
            .as_bytes()
            .cmp(other.path.as_os_str().as_bytes())
            .then_with(|| self.id.cmp(&other.id))
            .then_with(|| self.meta.cmp(&other.meta))
            .then_with(|| self.flag.cmp(&other.flag))
    }
}

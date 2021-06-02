use std::cmp;
use std::collections::BTreeSet;
use std::convert::TryFrom as _;
use std::fs;
use std::io;
use std::io::Write as _;
use std::os::unix::ffi::OsStrExt as _;
use std::path;

use byteorder::BigEndian;
use byteorder::WriteBytesExt as _;

use crate::file;
use crate::meta;
use crate::object;
use crate::util::Tap as _;

#[derive(Debug)]
pub struct Index {
    path: path::PathBuf,
}

impl Index {
    pub fn new(git: &path::Path) -> Self {
        Index {
            path: git.join("index"),
        }
    }

    pub fn lock(&mut self) -> io::Result<Lock> {
        let path = self.path.clone();
        Ok(Lock {
            index: self,
            lock: file::Lock::new(path)?,
            entries: BTreeSet::new(),
        })
    }
}

pub struct Lock<'index> {
    #[allow(unused)]
    index: &'index mut Index,
    lock: file::Lock,
    entries: BTreeSet<Entry>,
}

impl<'index> Lock<'index> {
    pub fn push(&mut self, meta: fs::Metadata, id: object::Id, path: path::PathBuf) {
        self.entries.insert(Entry::new(meta, id, path));
    }

    pub fn commit(mut self) -> io::Result<()> {
        let len = self
            .entries
            .len()
            .tap(u32::try_from)
            .expect("[INTERNAL ERROR]: more than 2^32 - 1 entries");

        let mut hash = sha1::Sha1::new();
        let mut buffer = Vec::new();
        let mut cursor = io::Cursor::new(&mut buffer);

        cursor.write_all(b"DIRC")?;
        cursor.write_u32::<BigEndian>(2)?;
        cursor.write_u32::<BigEndian>(len)?;
        hash.update(&buffer);
        self.lock.write_all(&buffer)?;

        for entry in &self.entries {
            buffer.clear();
            entry.write(io::Cursor::new(&mut buffer))?;
            hash.update(&buffer);
            self.lock.write_all(&buffer)?;
        }

        self.lock.write_all(&hash.digest().bytes())?;
        self.lock.commit()
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
    pub fn new(meta: fs::Metadata, id: object::Id, path: path::PathBuf) -> Self {
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

    fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        let path = self.path.as_os_str().as_bytes();

        let len = self.meta.len() + self.id.as_bytes().len() + 2 + path.len();
        let pad = 0b1000 - (len & 0b0111);

        self.meta.write(&mut writer)?;
        writer.write_all(self.id.as_bytes())?;
        writer.write_u16::<BigEndian>(self.flag)?;
        writer.write_all(path)?;
        for _ in 0..pad {
            writer.write_u8(0)?;
        }

        Ok(())
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

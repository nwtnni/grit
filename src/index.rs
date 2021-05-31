use std::cmp;
use std::convert::TryFrom;
use std::fs;
use std::io;
use std::io::Write as _;
use std::num;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path;

use byteorder::BigEndian;
use byteorder::WriteBytesExt as _;

use crate::file;
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
            entries: Vec::new(),
        })
    }
}

pub struct Lock<'index> {
    #[allow(unused)]
    index: &'index mut Index,
    lock: file::Lock,
    entries: Vec<Entry>,
}

impl<'index> Lock<'index> {
    pub fn push(&mut self, meta: fs::Metadata, id: object::Id, path: path::PathBuf) {
        self.entries.push(Entry::new(meta, id, path));
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

#[derive(Clone, Debug)]
pub struct Entry {
    meta: Metadata,
    id: object::Id,
    flag: u16,
    path: path::PathBuf,
}

impl Entry {
    pub fn new(meta: fs::Metadata, id: object::Id, path: path::PathBuf) -> Self {
        let meta = Metadata::try_from(meta).expect("[INTERNAL ERROR]: failed to convert metadata");
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

#[derive(Clone, Debug)]
struct Metadata {
    /// Change time (whole seconds)
    ctime: u32,
    /// Change time (fractional nanoseconds)
    ctime_nsec: u32,
    /// Modified time (whole seconds)
    mtime: u32,
    /// Modified time (fractional nanoseconds)
    mtime_nsec: u32,
    /// Device ID
    dev: u32,
    /// `inode` number
    ino: u32,
    /// Permission mode
    mode: u32,
    /// User ID
    uid: u32,
    /// Group ID
    gid: u32,
    /// File size (bytes)
    size: u32,
}

impl Metadata {
    fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_u32::<BigEndian>(self.ctime)?;
        writer.write_u32::<BigEndian>(self.ctime_nsec)?;
        writer.write_u32::<BigEndian>(self.mtime)?;
        writer.write_u32::<BigEndian>(self.mtime_nsec)?;
        writer.write_u32::<BigEndian>(self.dev)?;
        writer.write_u32::<BigEndian>(self.ino)?;
        writer.write_u32::<BigEndian>(self.mode)?;
        writer.write_u32::<BigEndian>(self.uid)?;
        writer.write_u32::<BigEndian>(self.gid)?;
        writer.write_u32::<BigEndian>(self.size)?;
        Ok(())
    }

    fn len(&self) -> usize {
        40
    }
}

impl TryFrom<fs::Metadata> for Metadata {
    type Error = num::TryFromIntError;
    fn try_from(meta: fs::Metadata) -> Result<Self, Self::Error> {
        Ok(Metadata {
            ctime: meta.ctime().tap(u32::try_from)?,
            ctime_nsec: meta.ctime_nsec().tap(u32::try_from)?,
            mtime: meta.mtime().tap(u32::try_from)?,
            mtime_nsec: meta.mtime_nsec().tap(u32::try_from)?,
            dev: meta.dev().tap(u32::try_from)?,
            ino: meta.ino().tap(u32::try_from)?,
            mode: if meta.mode() & 0o111 > 0 {
                0o100755
            } else {
                0o100644
            },
            uid: meta.uid(),
            gid: meta.gid(),
            size: meta.size().tap(u32::try_from)?,
        })
    }
}

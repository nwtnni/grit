use std::convert;
use std::fs;
use std::io;
use std::num;
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::fs::PermissionsExt as _;

use byteorder::BigEndian;
use byteorder::WriteBytesExt as _;

use crate::util::Tap as _;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Data {
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
    mode: Mode,
    /// User ID
    uid: u32,
    /// Group ID
    gid: u32,
    /// File size (bytes)
    size: u32,
}

impl Data {
    pub fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_u32::<BigEndian>(self.ctime)?;
        writer.write_u32::<BigEndian>(self.ctime_nsec)?;
        writer.write_u32::<BigEndian>(self.mtime)?;
        writer.write_u32::<BigEndian>(self.mtime_nsec)?;
        writer.write_u32::<BigEndian>(self.dev)?;
        writer.write_u32::<BigEndian>(self.ino)?;
        writer.write_u32::<BigEndian>(self.mode.as_u32())?;
        writer.write_u32::<BigEndian>(self.uid)?;
        writer.write_u32::<BigEndian>(self.gid)?;
        writer.write_u32::<BigEndian>(self.size)?;
        Ok(())
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        40
    }
}

impl convert::TryFrom<&'_ fs::Metadata> for Data {
    type Error = num::TryFromIntError;
    fn try_from(meta: &fs::Metadata) -> Result<Self, Self::Error> {
        Ok(Self {
            ctime: meta.ctime().tap(u32::try_from)?,
            ctime_nsec: meta.ctime_nsec().tap(u32::try_from)?,
            mtime: meta.mtime().tap(u32::try_from)?,
            mtime_nsec: meta.mtime_nsec().tap(u32::try_from)?,
            dev: meta.dev().tap(u32::try_from)?,
            ino: meta.ino().tap(u32::try_from)?,
            mode: Mode::from(meta),
            uid: meta.uid(),
            gid: meta.gid(),
            size: meta.size().tap(u32::try_from)?,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    Directory,
    Regular,
    Executable,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Directory => "40000",
            Mode::Regular => "100644",
            Mode::Executable => "100755",
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            Mode::Directory => 0o040000,
            Mode::Regular => 0o100644,
            Mode::Executable => 0o100755,
        }
    }
}

impl From<&'_ fs::Metadata> for Mode {
    fn from(meta: &fs::Metadata) -> Self {
        if meta.file_type().is_dir() {
            Mode::Directory
        } else if meta.permissions().mode() & 0o111 > 0 {
            Mode::Executable
        } else {
            Mode::Regular
        }
    }
}

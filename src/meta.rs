use std::convert;
use std::convert::TryFrom as _;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::num;
use std::os::unix::fs::MetadataExt as _;
use std::os::unix::fs::PermissionsExt as _;

use byteorder::BigEndian;
use byteorder::ReadBytesExt as _;
use byteorder::WriteBytesExt as _;

use crate::util::Tap as _;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Data {
    /// Change time (whole seconds)
    pub ctime: u32,
    /// Change time (fractional nanoseconds)
    pub ctime_nsec: u32,
    /// Modified time (whole seconds)
    pub mtime: u32,
    /// Modified time (fractional nanoseconds)
    pub mtime_nsec: u32,
    /// Device ID
    pub dev: u32,
    /// `inode` number
    pub ino: u32,
    /// Permission mode
    pub mode: Mode,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// File size (bytes)
    pub size: u32,
}

impl Data {
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        Ok(Data {
            ctime: reader.read_u32::<BigEndian>()?,
            ctime_nsec: reader.read_u32::<BigEndian>()?,
            mtime: reader.read_u32::<BigEndian>()?,
            mtime_nsec: reader.read_u32::<BigEndian>()?,
            dev: reader.read_u32::<BigEndian>()?,
            ino: reader.read_u32::<BigEndian>()?,
            mode: reader
                .read_u32::<BigEndian>()?
                .tap(Mode::try_from)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            uid: reader.read_u32::<BigEndian>()?,
            gid: reader.read_u32::<BigEndian>()?,
            size: reader.read_u32::<BigEndian>()?,
        })
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InvalidMode(u32);

impl fmt::Display for InvalidMode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Invalid mode {:#o}, expected 0o040000 or 0o100644 or 0o100755",
            self.0,
        )
    }
}

impl error::Error for InvalidMode {}

impl convert::TryFrom<u32> for Mode {
    type Error = InvalidMode;
    fn try_from(mode: u32) -> Result<Self, Self::Error> {
        match mode {
            0o040000 => Ok(Mode::Directory),
            0o100644 => Ok(Mode::Regular),
            0o100755 => Ok(Mode::Executable),
            invalid => Err(InvalidMode(invalid)),
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

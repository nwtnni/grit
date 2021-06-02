use std::convert;
use std::fs;
use std::io;
use std::num;
use std::os::unix::fs::MetadataExt as _;

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
    mode: u32,
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
        writer.write_u32::<BigEndian>(self.mode)?;
        writer.write_u32::<BigEndian>(self.uid)?;
        writer.write_u32::<BigEndian>(self.gid)?;
        writer.write_u32::<BigEndian>(self.size)?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        40
    }
}

impl convert::TryFrom<fs::Metadata> for Data {
    type Error = num::TryFromIntError;
    fn try_from(meta: fs::Metadata) -> Result<Self, Self::Error> {
        Ok(Self {
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

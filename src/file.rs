use std::fs;
use std::io;
use std::mem;
use std::path;

use rand::distributions;
use rand::Rng as _;
use sha1::Sha1;

use crate::util::Tap as _;

pub struct Checksum<T> {
    inner: T,
    hash: Sha1,
}

impl<T> Checksum<T> {
    pub fn new(inner: T) -> Self {
        Checksum {
            inner,
            hash: Sha1::new(),
        }
    }

    pub fn clear_checksum(&mut self) {
        self.hash.reset()
    }
}

impl<T: io::BufRead> io::BufRead for Checksum<T> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.inner.consume(amount)
    }
}

impl<T: io::Read> io::Read for Checksum<T> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(buffer)?;
        self.hash.update(&buffer[..len]);
        Ok(len)
    }
}

impl<T: io::Read> Checksum<T> {
    pub fn verify_checksum(mut self) -> io::Result<T> {
        let mut buffer = [0u8; 20];

        self.inner.read_exact(&mut buffer)?;

        if buffer != self.hash.digest().bytes() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Expected checksum {:?}, but found checksum {:?}",
                    buffer,
                    self.hash.digest().bytes(),
                ),
            ));
        }

        match self.inner.read(&mut buffer)? {
            0 => Ok(self.inner),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected byte after checksum: {:?}", buffer[0]),
            )),
        }
    }
}

impl<T: io::Write> io::Write for Checksum<T> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.hash.update(buffer);
        self.inner.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<T: io::Write> Checksum<T> {
    pub fn write_checksum(mut self) -> io::Result<T> {
        let digest = self.hash.digest().bytes();
        self.inner.write_all(&digest)?;
        Ok(self.inner)
    }
}

#[derive(Debug)]
pub struct Temp(Atomic);

impl Temp {
    pub fn new(target: path::PathBuf) -> io::Result<Self> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let source = b"tmp_obj_"
            .iter()
            .copied()
            .chain(
                rand::thread_rng()
                    .sample_iter(distributions::Alphanumeric)
                    .take(6),
            )
            .map(char::from)
            .collect::<String>()
            .tap(|name| target.with_file_name(name));

        Atomic::new(source, target).map(Self)
    }

    pub fn commit(self) -> io::Result<()> {
        self.0.commit()
    }
}

impl io::Write for Temp {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[derive(Debug)]
pub enum Lock {
    Write(WriteLock),
    ReadWrite(ReadWriteLock),
}

#[derive(Debug)]
pub struct WriteLock(Atomic);

impl WriteLock {
    pub fn new(target: path::PathBuf) -> io::Result<Self> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let source = target
            .clone()
            .into_os_string()
            .tap_mut(|path| path.push(".lock"))
            .tap(path::PathBuf::from);

        Atomic::new(source, target).map(Self)
    }

    pub fn read(self) -> io::Result<Lock> {
        let reader = match fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(&self.0.target)
        {
            Ok(file) => Some(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };

        match reader {
            None => Ok(Lock::Write(self)),
            Some(reader) => Ok(Lock::ReadWrite(ReadWriteLock {
                reader: Some(io::BufReader::new(reader)),
                writer: self.0,
            })),
        }
    }

    pub fn commit(self) -> io::Result<()> {
        self.0.commit()
    }
}

impl io::Write for WriteLock {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[derive(Debug)]
pub struct ReadWriteLock {
    reader: Option<io::BufReader<fs::File>>,
    writer: Atomic,
}

impl ReadWriteLock {
    pub fn commit(mut self) -> io::Result<()> {
        mem::take(&mut self.reader);
        self.writer.commit()
    }
}

impl io::BufRead for ReadWriteLock {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader
            .as_mut()
            .expect("[UNREACHABLE]: missing `ReadWriteLock` file")
            .fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.reader
            .as_mut()
            .expect("[UNREACHABLE]: missing `ReadWriteLock` file")
            .consume(amount)
    }
}

impl io::Read for ReadWriteLock {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.reader
            .as_mut()
            .expect("[UNREACHABLE]: missing `ReadWriteLock` file")
            .read(buffer)
    }
}

impl io::Write for ReadWriteLock {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.writer.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[derive(Debug)]
pub struct Atomic {
    source: path::PathBuf,
    target: path::PathBuf,
    file: Option<fs::File>,
}

impl Atomic {
    fn new(source: path::PathBuf, target: path::PathBuf) -> io::Result<Self> {
        let file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&source)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("Failed to open file: {} already exists", source.display()),
                ))
            }
            Err(error) => return Err(error),
        };

        Ok(Atomic {
            source,
            target,
            file: Some(file),
        })
    }

    fn commit(mut self) -> io::Result<()> {
        mem::take(&mut self.file);
        fs::rename(&self.source, &self.target)?;

        // Once we've successfully renamed the file, we want to avoid running our
        // destructor in case some other process has created the lock file in
        // between the `rename` and `drop` calls:
        //
        // - P0: acquire lock
        // - P0: fs::rename
        // - P1: acquire lock
        // - P0: mem::drop
        //
        // But in all other (error) paths, we **do** want to run the destructor so
        // that the lock is always released. Essentially, we want `rename` XOR `remove`.
        mem::take(&mut self.source);
        mem::take(&mut self.target);
        mem::forget(self);
        Ok(())
    }
}

impl Drop for Atomic {
    fn drop(&mut self) {
        // If `fs::rename` fails during `File::commit`, then it's possible that we've
        // already dropped `self.file`, but still need to remove `self.path` anyway,
        // which is why this is **not** in a conditional:
        //
        // ```
        // if let Some(_) = self.file.take() {
        //     fs::remove_file(&self.path).ok();
        // }
        // ```
        mem::take(&mut self.file);
        fs::remove_file(&self.source)
            .unwrap_or_else(|_| panic!("Failed to clean up file: {}", self.source.display()));
    }
}

impl io::Write for Atomic {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.file
            .as_mut()
            .expect("[UNREACHABLE]: missing `Atomic` file")
            .write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file
            .as_mut()
            .expect("[UNREACHABLE]: missing `Atomic` file")
            .flush()
    }
}

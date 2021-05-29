use std::fs;
use std::io;
use std::mem;
use std::ops;
use std::path;

use rand::distributions;
use rand::Rng as _;

use crate::util::Tap as _;

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

impl ops::Deref for Temp {
    type Target = Atomic;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Temp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub struct Lock(Atomic);

impl Lock {
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

    pub fn commit(self) -> io::Result<()> {
        self.0.commit()
    }
}

impl ops::Deref for Lock {
    type Target = Atomic;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Lock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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

impl ops::Deref for Atomic {
    type Target = fs::File;
    fn deref(&self) -> &Self::Target {
        self.file
            .as_ref()
            .expect("[UNREACHABLE]: missing underlying file")
    }
}

impl ops::DerefMut for Atomic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.file
            .as_mut()
            .expect("[UNREACHABLE]: missing underlying file")
    }
}

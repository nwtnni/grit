use std::fs;
use std::io;
use std::mem;
use std::ops;
use std::path;

use crate::util::Tap as _;

#[derive(Debug)]
pub struct File {
    path: path::PathBuf,
    file: Option<fs::File>,
}

impl File {
    pub fn new(path: path::PathBuf) -> io::Result<Self> {
        let path = match path.file_name() {
            Some(_) => path
                .into_os_string()
                .tap_mut(|path| path.push(".lock"))
                .tap(path::PathBuf::from),
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Expected path to file, but got {}", path.display()),
                ));
            }
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("Failed to acquire lock; {} already exists", path.display()),
                ))
            }
            Err(error) => return Err(error),
        };

        Ok(File {
            path,
            file: Some(file),
        })
    }

    pub fn commit(mut self) -> io::Result<()> {
        mem::take(&mut self.file);
        fs::rename(&self.path, &self.path.with_extension(""))?;

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
        mem::take(&mut self.path);
        mem::forget(self);
        Ok(())
    }
}

impl ops::Deref for File {
    type Target = fs::File;
    fn deref(&self) -> &Self::Target {
        self.file
            .as_ref()
            .expect("[UNREACHABLE]: missing underlying file")
    }
}

impl ops::DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.file
            .as_mut()
            .expect("[UNREACHABLE]: missing underlying file")
    }
}

impl Drop for File {
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
        match fs::remove_file(&self.path) {
            Ok(()) => (),
            error => error.expect(&format!(
                "Failed to release lock file: {}",
                self.path.display(),
            )),
        }
    }
}

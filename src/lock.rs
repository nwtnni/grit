use std::fs;
use std::io;
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
        let path = match path.is_file() {
            true => path
                .into_os_string()
                .tap_mut(|path| path.push(".lock"))
                .tap(path::PathBuf::from),
            false => {
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
        self.file.take();
        fs::rename(&self.path, &self.path.with_extension("")).expect("Failed to rename lock file");
    }
}

use std::fs;
use std::io;
use std::path;
use std::rc::Rc;

use crate::meta;
use crate::util;
use crate::util::Tap as _;

#[derive(Debug)]
pub struct Workspace {
    root: Rc<path::Path>,
}

impl Workspace {
    pub fn new(root: path::PathBuf) -> Self {
        Workspace {
            root: Rc::from(root),
        }
    }

    pub fn read(&self, relative: &path::Path) -> io::Result<Vec<u8>> {
        fs::read(self.root.join(relative))
    }

    pub fn root(&self) -> &path::Path {
        &self.root
    }

    pub fn walk_list(&self, relative: &path::Path) -> io::Result<util::Or<WalkFile, WalkList>> {
        self.walk(WalkList::new, relative)
    }

    pub fn walk_tree(&self, relative: &path::Path) -> io::Result<util::Or<WalkFile, WalkTree>> {
        self.walk(WalkTree::new, relative)
    }

    fn walk<F: for<'a> FnOnce(Rc<path::Path>, &'a path::Path) -> io::Result<W>, W>(
        &self,
        walker: F,
        relative: &path::Path,
    ) -> io::Result<util::Or<WalkFile, W>> {
        let root = Rc::clone(&self.root);
        let path = root.join(relative);
        let metadata = fs::metadata(&path)?;
        let file_type = metadata.file_type();

        if file_type.is_file() {
            Entry {
                root,
                path,
                metadata: meta::Metadata::from(metadata),
            }
            .tap(Option::Some)
            .tap(WalkFile)
            .tap(util::Or::L)
            .tap(Result::Ok)
        } else if file_type.is_dir() {
            walker(root, &path).map(util::Or::R)
        } else {
            unimplemented!("Unsupported file type: {:?}", file_type);
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entry {
    root: Rc<path::Path>,
    pub path: path::PathBuf,
    pub metadata: meta::Metadata,
}

impl Entry {
    pub fn path(&self) -> &path::Path {
        &self.path
    }

    pub fn relative_path(&self) -> &path::Path {
        self.path
            .strip_prefix(&*self.root)
            .expect("[INTERNAL ERROR]: workspace must contain entry")
    }

    pub fn metadata(&self) -> &meta::Metadata {
        &self.metadata
    }
}

#[derive(Debug)]
pub struct WalkList {
    root: Rc<path::Path>,
    iter: fs::ReadDir,
}

impl WalkList {
    pub fn new(root: Rc<path::Path>, path: &path::Path) -> io::Result<Self> {
        Ok(WalkList {
            root: Rc::clone(&root),
            iter: fs::read_dir(path)?,
        })
    }
}

impl Iterator for WalkList {
    type Item = io::Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = loop {
            match self.iter.next()? {
                Ok(entry)
                    if entry
                        .path()
                        .strip_prefix(&self.root)
                        .expect("[INTERNAL ERROR]: `WalkList` iterator not under root")
                        .starts_with(".git") =>
                {
                    continue;
                }
                Ok(entry) => break entry,
                Err(error) => return Some(Err(error)),
            };
        };

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return Some(Err(error)),
        };

        Some(Ok(Entry {
            root: Rc::clone(&self.root),
            path: entry.path(),
            metadata: meta::Metadata::from(metadata),
        }))
    }
}

#[derive(Debug)]
pub struct WalkFile(Option<Entry>);

impl Iterator for WalkFile {
    type Item = io::Result<Entry>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take().map(Result::Ok)
    }
}

#[derive(Debug)]
pub struct WalkTree {
    root: Rc<path::Path>,
    stack: Vec<fs::ReadDir>,
}

impl WalkTree {
    fn new(root: Rc<path::Path>, path: &path::Path) -> io::Result<Self> {
        Ok(WalkTree {
            root: Rc::clone(&root),
            stack: vec![fs::read_dir(path)?],
        })
    }
}

impl Iterator for WalkTree {
    type Item = io::Result<Entry>;
    fn next(&mut self) -> Option<Self::Item> {
        let entry = loop {
            match self.stack.last_mut()?.next() {
                Some(Ok(entry))
                    if entry
                        .path()
                        .strip_prefix(&self.root)
                        .expect("[INTERNAL ERROR]: `WalkTree` iterator not under root")
                        .starts_with(".git") =>
                {
                    continue;
                }
                Some(Ok(entry)) => break entry,
                Some(Err(error)) => return Some(Err(error)),
                None => {
                    self.stack.pop();
                }
            }
        };

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return Some(Err(error)),
        };

        let file_type = metadata.file_type();

        let entry = Entry {
            root: Rc::clone(&self.root),
            path: entry.path(),
            metadata: meta::Metadata::from(&metadata),
        };

        if file_type.is_file() {
            return Some(Ok(entry));
        }

        if !file_type.is_dir() {
            unimplemented!("Unsupported file type: {:?}", file_type);
        }

        match fs::read_dir(&entry.path) {
            Ok(iter) => self.stack.push(iter),
            Err(error) => return Some(Err(error)),
        }

        Some(Ok(entry))
    }
}

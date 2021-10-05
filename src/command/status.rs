use std::cmp;
use std::collections::btree_map;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::iter;
use std::ops;
use std::path;

use structopt::StructOpt;

use crate::meta;
use crate::object;
use crate::util;
use crate::util::Tap as _;
use crate::workspace;

#[derive(StructOpt)]
pub struct Configuration {}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let repository = crate::Repository::new(root);
        let status = Status {
            database: repository.database(),
            index: repository.index()?,
            references: repository.references(),
            workspace: repository.workspace(),
        };
        status.run()?;
        Ok(())
    }
}

struct Status {
    database: crate::Database,
    index: crate::Index,
    workspace: crate::Workspace,
    references: crate::References,
}

impl Status {
    fn run(self) -> anyhow::Result<()> {
        let head_commit = match self.references.read_head()? {
            None => return Ok(()),
            Some(head_commit) => head_commit,
        };

        let head = self.walk_head(&head_commit)?;
        let workspace = self.walk_workspace(path::Path::new("."))?;
        let change = self.detect_change(&head, &workspace)?;

        for (path, index_head_change, workspace_index_change) in &change {
            if let Some(change) = index_head_change {
                print!("{}", change);
            } else {
                print!(" ");
            }

            if let Some(change) = workspace_index_change {
                print!("{}", change);
            } else {
                print!(" ");
            }

            println!(" {}", path.display());
        }

        for path in &workspace.untracked {
            println!("?? {}", path.display());
        }

        Ok(())
    }

    fn walk_head(&self, tree: &object::Id) -> anyhow::Result<HeadState> {
        fn recurse(
            database: &crate::Database,
            tree: &object::Id,
            state: &mut HeadState,
        ) -> anyhow::Result<()> {
            match database.load(tree)? {
                crate::Object::Blob(_) => unreachable!(),
                crate::Object::Commit(commit) => recurse(database, commit.tree(), state),
                crate::Object::Tree(tree) => {
                    for node in tree {
                        if node.mode.is_directory() {
                            recurse(database, &node.id, state)?;
                        } else {
                            state.insert(util::PathBuf(node.path), (node.id, node.mode));
                        }
                    }
                    Ok(())
                }
            }
        }

        let mut state = HeadState::default();
        recurse(&self.database, tree, &mut state)?;
        Ok(state)
    }

    fn walk_workspace(&self, relative: &path::Path) -> anyhow::Result<WorkspaceState> {
        fn recurse(
            workspace: &crate::Workspace,
            index: &crate::Index,
            relative: &path::Path,
            state: &mut WorkspaceState,
        ) -> anyhow::Result<()> {
            for entry in workspace.walk_list(relative)? {
                let entry = entry?;
                let relative = entry.relative_path();
                let metadata = entry.metadata;

                match index.contains(relative) {
                    true if metadata.mode.is_directory() => {
                        recurse(workspace, index, relative, state)?
                    }
                    true => {
                        state
                            .tracked
                            .insert(relative.to_path_buf().tap(util::PathBuf), metadata);
                    }
                    false if is_trackable(workspace, index, &entry)? => {
                        let relative = if metadata.mode.is_directory() {
                            relative
                                .as_os_str()
                                .to_os_string()
                                .tap_mut(|path| path.push("/"))
                                .tap(path::PathBuf::from)
                        } else {
                            relative.to_path_buf()
                        };

                        state.untracked.insert(util::PathBuf(relative));
                    }
                    false => continue,
                }
            }
            Ok(())
        }

        fn is_trackable(
            workspace: &crate::Workspace,
            index: &crate::Index,
            entry: &workspace::Entry,
        ) -> anyhow::Result<bool> {
            let relative = entry.relative_path();

            if entry.metadata().mode.is_file() {
                return Ok(!index.contains(relative));
            }

            // FIXME: waiting on stabilization of [`Iterator::try_find`][tf]
            //
            // [tf]: https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.try_find
            for entry in workspace.walk_list(relative)? {
                if is_trackable(workspace, index, &entry?)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }

        let mut state = WorkspaceState::default();
        recurse(&self.workspace, &self.index, relative, &mut state)?;
        Ok(state)
    }

    fn detect_change(
        &self,
        head: &HeadState,
        workspace: &WorkspaceState,
    ) -> anyhow::Result<Change> {
        let mut change = Change::default();

        for entry in self.index.files() {
            match head.get(&entry.path() as &dyn util::Key) {
                Some((id, mode)) if mode == entry.metadata().mode() && id == entry.id() => (),
                Some(_) => change.insert_index_head(entry.path(), IndexHeadChange::Modified),
                None => change.insert_index_head(entry.path(), IndexHeadChange::Added),
            }

            let metadata = match workspace.tracked.get(&entry.path() as &dyn util::Key) {
                Some(metadata) => metadata,
                None => {
                    change.insert_workspace_index(entry.path(), WorkspaceIndexChange::Deleted);
                    continue;
                }
            };

            let old = entry.metadata();
            let new = metadata;

            if new.mode != old.mode || new.size != old.size {
                change.insert_workspace_index(entry.path(), WorkspaceIndexChange::Modified);
                continue;
            }

            let id = self
                .workspace
                .read(entry.path())
                .map(object::Blob::new)
                .map(object::Object::Blob)
                .map(|object| object.to_bytes())
                .map(|bytes| object::Id::hash(&bytes))?;

            if id != *entry.id() {
                change.insert_workspace_index(entry.path(), WorkspaceIndexChange::Modified);
            }
        }

        head.iter()
            .map(|(path, (_, _))| path)
            .filter(|path| !self.index.contains_file(path))
            .for_each(|path| change.insert_index_head(path, IndexHeadChange::Deleted));

        Ok(change)
    }
}

#[derive(Clone, Debug, Default)]
struct HeadState(BTreeMap<util::PathBuf, (object::Id, meta::Mode)>);

impl ops::Deref for HeadState {
    type Target = BTreeMap<util::PathBuf, (object::Id, meta::Mode)>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for HeadState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, Default)]
struct WorkspaceState {
    tracked: BTreeMap<util::PathBuf, meta::Metadata>,
    untracked: BTreeSet<util::PathBuf>,
}

#[derive(Clone, Debug, Default)]
struct Change {
    /// Changes between the index and the HEAD commit.
    index_head: BTreeMap<util::PathBuf, IndexHeadChange>,

    /// Changes between the workspace and the index.
    workspace_index: BTreeMap<util::PathBuf, WorkspaceIndexChange>,
}

impl Change {
    fn insert_index_head(&mut self, path: &path::Path, change: IndexHeadChange) {
        self.index_head
            .insert(path.to_path_buf().tap(util::PathBuf), change);
    }

    fn insert_workspace_index(&mut self, path: &path::Path, change: WorkspaceIndexChange) {
        self.workspace_index
            .insert(path.to_path_buf().tap(util::PathBuf), change);
    }
}

impl<'a> IntoIterator for &'a Change {
    type Item = <ChangeIter<'a> as Iterator>::Item;
    type IntoIter = ChangeIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ChangeIter {
            index_head: self.index_head.iter().peekable(),
            workspace_index: self.workspace_index.iter().peekable(),
        }
    }
}

#[derive(Clone, Debug)]
struct ChangeIter<'a> {
    index_head: iter::Peekable<btree_map::Iter<'a, util::PathBuf, IndexHeadChange>>,
    workspace_index: iter::Peekable<btree_map::Iter<'a, util::PathBuf, WorkspaceIndexChange>>,
}

impl<'a> Iterator for ChangeIter<'a> {
    type Item = (
        &'a path::Path,
        Option<IndexHeadChange>,
        Option<WorkspaceIndexChange>,
    );
    fn next(&mut self) -> Option<Self::Item> {
        let (index_head_path, index_head_change, workspace_index_path, workspace_index_change) =
            match (
                self.index_head.peek().copied(),
                self.workspace_index.peek().copied(),
            ) {
                (None, None) => return None,
                (Some((index_head_path, index_head_change)), None) => {
                    self.index_head.next();
                    return Some((index_head_path, Some(*index_head_change), None));
                }
                (None, Some((workspace_index_path, workspace_index_change))) => {
                    self.workspace_index.next();
                    return Some((workspace_index_path, None, Some(*workspace_index_change)));
                }
                (
                    Some((index_head_path, index_head_change)),
                    Some((workspace_index_path, workspace_index_change)),
                ) => (
                    &*index_head_path,
                    *index_head_change,
                    &*workspace_index_path,
                    *workspace_index_change,
                ),
            };

        match index_head_path.cmp(&workspace_index_path) {
            cmp::Ordering::Less => {
                self.index_head.next();
                Some((index_head_path, Some(index_head_change), None))
            }
            cmp::Ordering::Equal => {
                self.index_head.next();
                self.workspace_index.next();
                Some((
                    index_head_path,
                    Some(index_head_change),
                    Some(workspace_index_change),
                ))
            }
            cmp::Ordering::Greater => {
                self.workspace_index.next();
                Some((workspace_index_path, None, Some(workspace_index_change)))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum IndexHeadChange {
    Added,
    Deleted,
    Modified,
}

impl fmt::Display for IndexHeadChange {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexHeadChange::Added => write!(fmt, "A"),
            IndexHeadChange::Deleted => write!(fmt, "D"),
            IndexHeadChange::Modified => write!(fmt, "M"),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum WorkspaceIndexChange {
    Deleted,
    Modified,
}

impl fmt::Display for WorkspaceIndexChange {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceIndexChange::Deleted => write!(fmt, "D"),
            WorkspaceIndexChange::Modified => write!(fmt, "M"),
        }
    }
}

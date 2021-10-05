use std::cmp;
use std::collections::btree_map;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env;
use std::io::Write as _;
use std::iter;
use std::ops;
use std::path;

use structopt::StructOpt;
use termcolor::WriteColor as _;

use crate::meta;
use crate::object;
use crate::util;
use crate::util::Tap as _;
use crate::workspace;

#[derive(StructOpt)]
pub struct Configuration {
    #[structopt(long)]
    porcelain: bool,
}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let repository = crate::Repository::new(root);
        let stdout = termcolor::StandardStream::stdout(match isatty::stdout_isatty() {
            true => termcolor::ColorChoice::Always,
            false => termcolor::ColorChoice::Never,
        });

        let status = Status {
            database: repository.database(),
            index: repository.index()?,
            references: repository.references(),
            workspace: repository.workspace(),
            stdout: stdout.lock(),
        };

        status.run(self.porcelain)?;

        Ok(())
    }
}

struct Status<'a> {
    database: crate::Database,
    index: crate::Index,
    workspace: crate::Workspace,
    references: crate::References,
    stdout: termcolor::StandardStreamLock<'a>,
}

impl Status<'_> {
    fn run(mut self, porcelain: bool) -> anyhow::Result<()> {
        let head_commit = match self.references.read_head()? {
            None => return Ok(()),
            Some(head_commit) => head_commit,
        };

        let head = self.walk_head(&head_commit)?;
        let workspace = self.walk_workspace(path::Path::new("."))?;
        let changes = self.detect_changes(&head, &workspace)?;

        if porcelain {
            self.print_porcelain(&changes, &workspace)?;
        } else {
            self.print_pretty(&changes, &workspace)?;
        }

        Ok(())
    }

    fn print_porcelain(
        &mut self,
        changes: &Changes,
        workspace: &WorkspaceState,
    ) -> anyhow::Result<()> {
        for (path, index_head_change, workspace_index_change) in changes {
            writeln!(
                &mut self.stdout,
                "{}{} {}",
                index_head_change
                    .map(IndexHeadChange::into_porcelain)
                    .unwrap_or(" "),
                workspace_index_change
                    .map(WorkspaceIndexChange::into_porcelain)
                    .unwrap_or(" "),
                path.display(),
            )?;
        }

        for path in &workspace.untracked {
            writeln!(&mut self.stdout, "?? {}", path.display())?;
        }

        Ok(())
    }

    fn print_pretty(
        &mut self,
        changes: &Changes,
        workspace: &WorkspaceState,
    ) -> anyhow::Result<()> {
        self.print_change_set(
            termcolor::Color::Green,
            |change| Some(change.into_pretty()),
            "Changes to be committed:\n  \
                (use \"git restore --staged <file>...\" to unstage)",
            &changes.index_head,
        )?;

        self.print_change_set(
            termcolor::Color::Red,
            |change| Some(change.into_pretty()),
            "Changes not staged for commit:\n  \
                (use \"git add/rm <file>...\" to update what will be committed)\n  \
                (use \"git restore <file>...\" to discard changes in working directory)",
            &changes.workspace_index,
        )?;

        self.print_change_set(
            termcolor::Color::Red,
            |()| None,
            "Untracked files:\n  \
                (use \"git add <file>...\" to include in what will be committed)",
            workspace.untracked.iter().map(|path| (path, ())),
        )?;

        if !changes.index_head.is_empty() {
            return Ok(());
        }

        if !changes.workspace_index.is_empty() {
            writeln!(
                &mut self.stdout,
                "no changes added to commit (use \"git add\" and/or \"git commit -a\")"
            )?;
        } else if !workspace.untracked.is_empty() {
            writeln!(
                &mut self.stdout,
                "nothing added to commit but untracked files present (use \"git add\" to track)"
            )?;
        } else {
            writeln!(&mut self.stdout, "nothing to commit, working tree clean")?;
        }

        Ok(())
    }

    fn print_change_set<'a, 'b, I, T>(
        &mut self,
        color: termcolor::Color,
        display: fn(T) -> Option<&'static str>,
        message: &'a str,
        into_iter: I,
    ) -> anyhow::Result<()>
    where
        I: IntoIterator<Item = (&'b util::PathBuf, T)>,
    {
        let mut iter = into_iter.into_iter().peekable();
        if iter.peek().is_none() {
            return Ok(());
        }

        writeln!(&mut self.stdout, "{}", message)?;
        self.stdout
            .set_color(&termcolor::ColorSpec::new().set_fg(Some(color)))?;

        for (path, status) in iter {
            match display(status) {
                Some(status) => write!(&mut self.stdout, "\t{:12}", status)?,
                None => write!(&mut self.stdout, "\t")?,
            }
            writeln!(&mut self.stdout, "{}", path.display())?;
        }

        writeln!(&mut self.stdout)?;
        self.stdout.reset()?;
        Ok(())
    }

    fn walk_head(&self, tree: &object::Id) -> anyhow::Result<HeadState> {
        fn recurse(
            database: &crate::Database,
            tree: &object::Id,
            state: &mut HeadState,
            prefix: &mut path::PathBuf,
        ) -> anyhow::Result<()> {
            match database.load(tree)? {
                crate::Object::Blob(_) => unreachable!(),
                crate::Object::Commit(commit) => recurse(database, commit.tree(), state, prefix),
                crate::Object::Tree(tree) => {
                    for node in tree {
                        if node.mode.is_directory() {
                            prefix.push(&node.path);
                            recurse(database, &node.id, state, prefix)?;
                            prefix.pop();
                        } else {
                            state.insert(
                                util::PathBuf(prefix.join(node.path)),
                                (node.id, node.mode),
                            );
                        }
                    }
                    Ok(())
                }
            }
        }

        let mut state = HeadState::default();
        let mut prefix = path::PathBuf::default();
        recurse(&self.database, tree, &mut state, &mut prefix)?;
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

    fn detect_changes(
        &self,
        head: &HeadState,
        workspace: &WorkspaceState,
    ) -> anyhow::Result<Changes> {
        let mut changes = Changes::default();

        for entry in self.index.files() {
            match head.get(&entry.path() as &dyn util::Key) {
                Some((id, mode)) if mode == entry.metadata().mode() && id == entry.id() => (),
                Some(_) => changes.insert_index_head(entry.path(), IndexHeadChange::Modified),
                None => changes.insert_index_head(entry.path(), IndexHeadChange::Added),
            }

            let metadata = match workspace.tracked.get(&entry.path() as &dyn util::Key) {
                Some(metadata) => metadata,
                None => {
                    changes.insert_workspace_index(entry.path(), WorkspaceIndexChange::Deleted);
                    continue;
                }
            };

            let old = entry.metadata();
            let new = metadata;

            if new.mode != old.mode || new.size != old.size {
                changes.insert_workspace_index(entry.path(), WorkspaceIndexChange::Modified);
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
                changes.insert_workspace_index(entry.path(), WorkspaceIndexChange::Modified);
            }
        }

        head.iter()
            .map(|(path, (_, _))| path)
            .filter(|path| !self.index.contains_file(path))
            .for_each(|path| changes.insert_index_head(path, IndexHeadChange::Deleted));

        Ok(changes)
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
struct Changes {
    /// Changes between the index and the HEAD commit.
    index_head: BTreeMap<util::PathBuf, IndexHeadChange>,

    /// Changes between the workspace and the index.
    workspace_index: BTreeMap<util::PathBuf, WorkspaceIndexChange>,
}

impl Changes {
    fn insert_index_head(&mut self, path: &path::Path, change: IndexHeadChange) {
        self.index_head
            .insert(path.to_path_buf().tap(util::PathBuf), change);
    }

    fn insert_workspace_index(&mut self, path: &path::Path, change: WorkspaceIndexChange) {
        self.workspace_index
            .insert(path.to_path_buf().tap(util::PathBuf), change);
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = <ChangesIter<'a> as Iterator>::Item;
    type IntoIter = ChangesIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ChangesIter {
            index_head: self.index_head.iter().peekable(),
            workspace_index: self.workspace_index.iter().peekable(),
        }
    }
}

#[derive(Clone, Debug)]
struct ChangesIter<'a> {
    index_head: iter::Peekable<btree_map::Iter<'a, util::PathBuf, IndexHeadChange>>,
    workspace_index: iter::Peekable<btree_map::Iter<'a, util::PathBuf, WorkspaceIndexChange>>,
}

impl<'a> Iterator for ChangesIter<'a> {
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

impl IndexHeadChange {
    fn into_porcelain(self) -> &'static str {
        match self {
            IndexHeadChange::Added => "A",
            IndexHeadChange::Deleted => "D",
            IndexHeadChange::Modified => "M",
        }
    }

    fn into_pretty(self) -> &'static str {
        match self {
            IndexHeadChange::Added => "new file:",
            IndexHeadChange::Deleted => "deleted:",
            IndexHeadChange::Modified => "modified:",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum WorkspaceIndexChange {
    Deleted,
    Modified,
}

impl WorkspaceIndexChange {
    fn into_porcelain(self) -> &'static str {
        match self {
            WorkspaceIndexChange::Deleted => "D",
            WorkspaceIndexChange::Modified => "M",
        }
    }

    fn into_pretty(self) -> &'static str {
        match self {
            WorkspaceIndexChange::Deleted => "deleted:",
            WorkspaceIndexChange::Modified => "modified:",
        }
    }
}

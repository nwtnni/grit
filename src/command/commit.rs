use std::env;
use std::io;
use std::io::Read as _;
use std::path;

use structopt::StructOpt;

use crate::index;
use crate::object;
use crate::object::commit;
use crate::object::tree;
use crate::util::Tap as _;

#[derive(StructOpt)]
pub struct Configuration {
    #[structopt(long, env = "GIT_AUTHOR_NAME")]
    author_name: String,

    #[structopt(long, env = "GIT_AUTHOR_EMAIL")]
    author_email: String,

    #[structopt(short, long)]
    message: Option<String>,
}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let message = match self.message {
            Some(message) => message,
            None => {
                let stdin = io::stdin();
                let mut stdin = stdin.lock();
                let mut buffer = String::new();
                stdin.read_to_string(&mut buffer)?;
                buffer
            }
        };

        let root = env::current_dir()?;
        let git = root.join(".git");

        let database = crate::Database::new(&git)?;
        let reference = crate::Reference::new(&git);
        let index = crate::Index::lock(&git)?;

        let commit = Commit {
            database,
            index,
            reference,
            author_name: self.author_name,
            author_email: self.author_email,
            message,
        };

        commit.run()?;
        Ok(())
    }
}

struct Commit {
    database: crate::Database,
    index: crate::Index,
    reference: crate::Reference,
    author_name: String,
    author_email: String,
    message: String,
}

impl Commit {
    pub fn run(self) -> io::Result<()> {
        let mut stack = Vec::new();
        let mut count = Vec::new();

        for node in &self.index {
            let path = node.path();
            let depth = path.components().count();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_os_string()
                .tap(path::PathBuf::from);

            let id = match node {
                index::Node::File(entry) => {
                    count.resize(depth, 0);
                    *entry.id()
                }
                index::Node::Directory(_) => {
                    count.resize(depth + 1, 0);
                    let index = match count.pop() {
                        None => unreachable!(),
                        Some(0) => continue,
                        Some(count) => stack.len() - count,
                    };
                    stack
                        .split_off(index)
                        .tap(object::Tree::new)
                        .tap(crate::Object::Tree)
                        .tap(|tree| self.database.store(&tree))?
                }
            };

            let mode = node.mode();
            let node = tree::Node::new(name, id, *mode);

            stack.push(node);
            count.last_mut().map(|count| *count += 1);
        }

        let commit_header = self
            .message
            .split('\n')
            .next()
            .unwrap_or_default()
            .to_owned();
        let commit_tree = *stack.pop().unwrap().id();

        let author = commit::Author::new(self.author_name, self.author_email, chrono::Local::now());
        let parent = self.reference.head()?;
        let commit = crate::Object::Commit(object::Commit::new(
            commit_tree,
            parent,
            author,
            self.message,
        ));
        let commit_id = self.database.store(&commit)?;

        self.reference.set_head(&commit_id)?;

        println!(
            "[{}{}] {}",
            if parent.is_some() {
                ""
            } else {
                "(root-commit)"
            },
            commit_id,
            commit_header
        );

        Ok(())
    }
}

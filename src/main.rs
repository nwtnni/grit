use std::env;
use std::fs;
use std::io;
use std::io::Read as _;
use std::path;

use grit::command;
use grit::index;
use grit::object;
use grit::object::commit;
use grit::object::tree;
use grit::util::Tap as _;
use grit::Object;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Command {
    Add(command::Add),
    Commit(Commit),
    Init(Init),
}

#[derive(StructOpt)]
struct Commit {
    #[structopt(long, env = "GIT_AUTHOR_NAME")]
    author_name: String,

    #[structopt(long, env = "GIT_AUTHOR_EMAIL")]
    author_email: String,

    #[structopt(short, long)]
    message: Option<String>,
}

/// Initialize a new git repository.
#[derive(StructOpt)]
struct Init {
    /// Path to directory to initialize.
    ///
    /// Default to current working directory if not provided.
    root: Option<path::PathBuf>,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    match Command::from_args() {
        Command::Add(add) => add.run(),
        Command::Commit(Commit {
            author_name,
            author_email,
            message,
        }) => {
            let message = match message {
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

            let database = grit::Database::new(&git)?;
            let reference = grit::Reference::new(&git);
            let index = grit::Index::lock(&git)?;

            let mut stack = Vec::new();
            let mut count = Vec::new();

            for node in &index {
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
                            .tap(Object::Tree)
                            .tap(|tree| database.store(&tree))?
                    }
                };

                let mode = node.mode();
                let node = tree::Node::new(name, id, *mode);

                stack.push(node);
                count.last_mut().map(|count| *count += 1);
            }

            let commit_header = message.split('\n').next().unwrap_or_default().to_owned();
            let commit_tree = *stack.pop().unwrap().id();

            let author = commit::Author::new(author_name, author_email, chrono::Local::now());
            let parent = reference.head()?;
            let commit = Object::Commit(object::Commit::new(commit_tree, parent, author, message));
            let commit_id = database.store(&commit)?;

            reference.set_head(&commit_id)?;

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
        Command::Init(Init { root }) => {
            let root = match root {
                None => env::current_dir()?,
                Some(root) => {
                    fs::create_dir_all(&root)?;
                    root.canonicalize()?
                }
            };

            let mut path = root.join(".git");

            for directory in &["objects", "refs"] {
                path.push(directory);
                fs::create_dir_all(&path)?;
                path.pop();
            }

            log::info!("Initialized empty git repository at `{}`", root.display());

            Ok(())
        }
    }
}

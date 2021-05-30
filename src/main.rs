use std::env;
use std::fs;
use std::io;
use std::io::Read as _;
use std::path;

use grit::object;
use grit::object::commit;
use grit::object::tree;
use grit::Object;
use grit::util::Tap as _;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Command {
    Add(Add),
    Commit(Commit),
    Init(Init),
}

#[derive(StructOpt)]
struct Add {
    path: path::PathBuf,
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
        Command::Add(Add { path: _ }) => {
            todo!()
        }
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

            let workspace = grit::Workspace::new(root);
            let database = grit::Database::new(&git)?;
            let reference = grit::Reference::new(&git);

            let mut stack = Vec::new();
            let mut count = Vec::new();

            for entry in &workspace {
                let entry = entry?;
                let path = entry.path();
                let meta = path.metadata()?;
                let name = path
                    .file_name()
                    .expect("[UNREACHABLE]: file must have name")
                    .to_os_string()
                    .tap(path::PathBuf::from);

                let depth = path
                    .strip_prefix(workspace.root())
                    .expect("[UNREACHABLE]: file must be in workspace")
                    .components()
                    .count();

                let object = if meta.file_type().is_file() {
                    count.resize(depth, 0);
                    fs::read(&path).map(object::Blob::new).map(Object::Blob)?
                } else if meta.file_type().is_dir() {
                    count.resize(depth + 1, 0);
                    let index = match count.pop() {
                        None => unreachable!(),
                        Some(0) => continue,
                        Some(count) => stack.len() - count,
                    };
                    Object::Tree(object::Tree::new(stack.split_off(index)))
                } else {
                    unimplemented!("Unsupported file type: {:?}", meta.file_type());
                };

                let id = database.store(&object)?;
                let node = tree::Node::new(name, id, meta);
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

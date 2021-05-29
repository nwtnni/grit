use std::env;
use std::fs;
use std::io;
use std::io::Read as _;
use std::path;

use grit::object;
use grit::object::commit;
use grit::object::tree;
use grit::Object;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Command {
    Init(Init),
    Commit(Commit),
}

/// Initialize a new git repository.
#[derive(StructOpt)]
struct Init {
    /// Path to directory to initialize.
    ///
    /// Default to current working directory if not provided.
    root: Option<path::PathBuf>,
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

fn main() -> anyhow::Result<()> {
    env_logger::init();

    match Command::from_args() {
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
            let objects = git.join("objects");
            let head = git.join("HEAD");

            let workspace = grit::Workspace::new(root);
            let database = grit::Database::new(objects);

            let mut nodes = Vec::new();

            for path in workspace.files() {
                let path = path?;
                let blob = fs::read(&path).map(object::Blob::new).map(Object::Blob)?;
                let blob_id = database.store(&blob)?;

                let relative = path
                    .strip_prefix(workspace.root())
                    .expect("[UNREACHABLE]: workspace root is always prefix of path")
                    .to_path_buf();

                nodes.push(tree::Node::new(relative, blob_id));
            }

            let tree = Object::Tree(object::Tree::new(nodes));
            let tree_id = database.store(&tree)?;

            let commit_header = message.split('\n').next().unwrap_or_default().to_owned();

            let author = commit::Author::new(author_name, author_email, chrono::Local::now());
            let commit = Object::Commit(object::Commit::new(tree_id, author, message));
            let commit_id = database.store(&commit)?;

            fs::write(head, commit_id.to_string())?;

            println!("[(root-commit) {}] {}", commit_id, commit_header);

            Ok(())
        }
    }
}

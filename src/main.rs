use std::env;
use std::fs;
use std::path;

use grit::object;
use grit::object::tree;
use grit::object::Tree;
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
struct Commit {}

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
        Command::Commit(Commit {}) => {
            let root = env::current_dir()?;
            let git = root.join(".git");
            let objects = git.join("objects");

            let workspace = grit::Workspace::new(root);
            let database = grit::Database::new(objects);

            let mut nodes = Vec::new();

            for path in workspace.files() {
                let path = path?;
                let blob = fs::read(&path).map(object::Blob::new).map(Object::Blob)?;
                let data = blob.encode();
                let id = object::Id::from(&data);

                database.store(&id, &data)?;

                let relative = path
                    .strip_prefix(workspace.root())
                    .expect("[UNREACHABLE]: workspace root is always prefix of path")
                    .to_path_buf();

                nodes.push(tree::Node::new(relative, id));
            }

            let tree = Object::Tree(Tree::new(nodes));
            let data = tree.encode();
            let id = object::Id::from(&data);
            database.store(&id, &data)?;

            Ok(())
        }
    }
}

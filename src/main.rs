use std::env;
use std::fs;
use std::path;

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
            let workspace = grit::Workspace::new(root);
            for path in workspace.files() {
                dbg!(path?);
            }
            Ok(())
        }
    }
}

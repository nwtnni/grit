use std::env;
use std::fs;
use std::path;

use grit::command;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Command {
    Add(command::Add),
    Commit(command::Commit),
    Init(Init),
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
        Command::Commit(commit) => commit.run(),
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

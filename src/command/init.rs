use std::env;
use std::fs;
use std::path;

use structopt::StructOpt;

/// Initialize a new git repository.
#[derive(StructOpt)]
pub struct Init {
    /// Path to directory to initialize.
    ///
    /// Default to current working directory if not provided.
    root: Option<path::PathBuf>,
}

impl Init {
    pub fn run(self) -> anyhow::Result<()> {
        let root = match self.root {
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
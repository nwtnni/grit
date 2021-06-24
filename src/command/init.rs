use std::env;
use std::fs;
use std::io;
use std::path;

use structopt::StructOpt;

/// Initialize a new git repository.
#[derive(StructOpt)]
pub struct Configuration {
    /// Path to directory to initialize.
    ///
    /// Default to current working directory if not provided.
    root: Option<path::PathBuf>,
}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = match self.root {
            None => env::current_dir()?,
            Some(root) => {
                fs::create_dir_all(&root)?;
                root.canonicalize()?
            }
        };

        let init = Init { root };
        init.run()?;
        Ok(())
    }
}

struct Init {
    root: path::PathBuf,
}

impl Init {
    fn run(mut self) -> io::Result<()> {
        self.root.push(".git");
        for directory in &["objects", "refs"] {
            self.root.push(directory);
            fs::create_dir_all(&self.root)?;
            self.root.pop();
        }
        self.root.pop();

        log::info!(
            "Initialized empty git repository at `{}`",
            self.root.display()
        );

        Ok(())
    }
}

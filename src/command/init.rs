use std::env;
use std::fs;
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

        let repository = crate::Repository::new(root);
        let init = Init { repository };
        init.run()?;
        Ok(())
    }
}

struct Init {
    repository: crate::Repository,
}

impl Init {
    fn run(mut self) -> anyhow::Result<()> {
        self.repository.init()?;

        log::info!(
            "Initialized empty git repository at `{}`",
            self.repository.root().display()
        );

        Ok(())
    }
}

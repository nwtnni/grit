use std::env;

use anyhow::anyhow;
use structopt::StructOpt;

use crate::object;
use crate::object::Object;

#[derive(StructOpt)]
pub struct Configuration {
    id: Option<object::Id>,
}

impl Configuration {
    pub fn run(self) -> anyhow::Result<()> {
        let root = env::current_dir()?;
        let repository = crate::Repository::new(root);
        let show = Show {
            database: repository.database(),
            references: repository.references(),
            id: self.id,
        };
        show.run()?;
        Ok(())
    }
}

struct Show {
    database: crate::Database,
    references: crate::References,
    id: Option<object::Id>,
}

impl Show {
    fn run(self) -> anyhow::Result<()> {
        if let Some(id) = &self.id {
            return self.show_tree(id);
        }

        let head = self
            .references
            .read_head()?
            .ok_or_else(|| anyhow!("Expected HEAD commit"))?;

        let commit = match self.database.load(&head)? {
            Object::Blob(_) | Object::Tree(_) => unreachable!(),
            Object::Commit(commit) => commit,
        };

        self.show_tree(commit.tree())
    }

    fn show_tree(&self, id: &object::Id) -> anyhow::Result<()> {
        let tree = match self.database.load(id)? {
            Object::Blob(_) => unreachable!(),
            Object::Commit(_) => unreachable!(),
            Object::Tree(tree) => tree,
        };

        for node in &tree {
            if node.mode.is_directory() {
                self.show_tree(&node.id)?;
            } else {
                println!("{} {} {}", node.mode.as_str(), node.id, node.path.display());
            }
        }

        Ok(())
    }
}

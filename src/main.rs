use grit::command;
use structopt::StructOpt;

#[derive(StructOpt)]
enum Command {
    Add(command::Add),
    Commit(command::Commit),
    Init(command::Init),
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    match Command::from_args() {
        Command::Add(add) => add.run(),
        Command::Commit(commit) => commit.run(),
        Command::Init(init) => init.run(),
    }
}

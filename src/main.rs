mod command;
mod parser;
mod shell;
mod state;

use anyhow::Result;
use shell::Pshell;

fn main() -> Result<()> {
    let mut shell = Pshell::new()?;

    shell.run()?;

    Ok(())
}

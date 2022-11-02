mod command;
mod parser;
mod server;
mod shell;
mod state;

use anyhow::Result;
use server::start_services;
use shell::Pshell;

#[tokio::main]
async fn main() -> Result<()> {
    let mut shell = Pshell::new().await?;
    tokio::spawn(start_services(shell.get_state_ref()));

    shell.run().await?;

    Ok(())
}

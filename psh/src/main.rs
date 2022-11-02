mod command;
mod parser;
mod server;
mod shell;
mod state;

use anyhow::Result;
use server::start_env_service;
use shell::Pshell;

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(start_env_service());
    let mut shell = Pshell::new().await?;

    shell.run().await?;

    Ok(())
}

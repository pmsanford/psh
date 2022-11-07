mod command;
mod parser;
mod plugins;
mod server;
mod shell;
mod state;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use nix::libc::SIGINT;
use plugins::get_prompt;
use server::start_services;
use shell::Pshell;
use signal_hook_tokio::Signals;
use state::State;
use tokio_stream::StreamExt;

struct SigHandler {
    state: Arc<Mutex<State>>,
}

impl SigHandler {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }

    pub async fn handle_signals(&mut self) -> Result<()> {
        let mut signals = Signals::new(&[SIGINT])?;
        signals.handle();
        while let Some(_sigint) = signals.next().await {
            let id = self.state.lock().unwrap().running_pid;
            if let Some(id) = id {
                let pid = nix::unistd::Pid::from_raw(id as i32);
                if let Err(e) = nix::sys::signal::kill(pid, nix::sys::signal::SIGINT) {
                    eprintln!("Couldn't send signal! {}", e);
                }
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Prompt: {}", get_prompt()?);
    let mut shell = Pshell::new().await?;
    tokio::spawn(start_services(shell.get_state_ref()));
    let sighandler = Box::leak(Box::new(SigHandler::new(shell.get_state_ref())));
    tokio::spawn(sighandler.handle_signals());

    shell.run().await?;

    Ok(())
}

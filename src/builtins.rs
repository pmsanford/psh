use std::{env, path::Path};

use crate::parser::Command;
use anyhow::Result;

pub fn run_builtin(cmd: &Command) -> Result<bool> {
    Ok(match cmd.command.as_str() {
        "cd" => {
            let newpath = cmd
                .args
                .first()
                .cloned()
                .or_else(|| env::var("HOME").ok())
                .unwrap_or_else(|| String::from("/"));
            let newpath = Path::new(&newpath);
            if let Err(e) = env::set_current_dir(newpath) {
                eprintln!("{}", e);
            }
            true
        }
        _ => false,
    })
}

pub fn is_exit(cmd: &Command) -> Result<bool> {
    Ok(cmd.command == "exit")
}

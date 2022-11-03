use std::{collections::HashMap, path::PathBuf};

use crate::command::Arg;

#[derive(Debug, Clone)]
pub struct Alias {
    pub alias: String,
    pub command: String,
    pub args: Vec<Arg>,
}

impl Alias {
    pub fn display(&self) -> String {
        format!(
            "{} -> {} {}",
            self.alias,
            self.command,
            self.args
                .iter()
                .map(|a| format!("{}", a))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

#[derive(Debug)]
pub struct State {
    pub aliases: HashMap<String, Alias>,
    pub history_path: PathBuf,
    pub current_command: Option<String>,
    pub running_pid: Option<u32>,
}

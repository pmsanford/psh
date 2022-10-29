use std::{collections::HashMap, path::PathBuf};

use crate::command::Arg;

#[derive(Debug)]
pub struct Alias {
    pub alias: String,
    pub command: String,
    pub args: Vec<Arg>,
}

pub struct State {
    pub aliases: HashMap<String, Alias>,
    pub history_path: PathBuf,
}

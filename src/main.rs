use std::{collections::HashMap, env, fs::File, io::Read, path::PathBuf, process::Stdio};

mod command;
mod parser;
mod state;

use anyhow::Result;
use directories::{ProjectDirs, UserDirs};
use once_cell::sync::OnceCell;
use owo_colors::OwoColorize;
use parser::{parse_alias, parse_pest};
use rustyline::{CompletionType, Editor};
use state::{Alias, State};

static mut STATE: OnceCell<State> = OnceCell::new();

fn path_prompt() -> Result<String> {
    let ud = UserDirs::new().ok_or_else(|| anyhow::anyhow!("Couldn't find user dirs"))?;
    let home = ud.home_dir();

    let pwd = env::current_dir()?;

    if pwd.starts_with(home) {
        Ok(format!(
            "~/{}",
            pwd.strip_prefix(home)?.to_string_lossy().yellow()
        ))
    } else {
        Ok(format!(
            "{}",
            pwd.to_string_lossy().to_string().bright_blue()
        ))
    }
}

fn load_state() -> Result<State> {
    let aliases = load_aliases()?;
    Ok(State { aliases })
}

fn load_aliases() -> Result<HashMap<String, Alias>> {
    let pd = ProjectDirs::from("net", "paulsanford", "psh").expect("project dirs");
    let mut aliases = PathBuf::from(pd.config_dir());
    aliases.push("aliases");

    if !aliases.exists() {
        return Ok(HashMap::new());
    }

    let mut file = File::open(aliases)?;
    let mut cont = String::new();
    file.read_to_string(&mut cont)?;

    let mut hm = HashMap::new();

    for line in cont.lines() {
        let alias = parse_alias(line)?;
        hm.insert(alias.alias.clone(), alias);
    }

    Ok(hm)
}

fn main() -> Result<()> {
    let config = rustyline::Config::builder()
        .max_history_size(100)
        .auto_add_history(true)
        .history_ignore_space(true)
        .history_ignore_dups(true)
        .completion_type(CompletionType::List)
        .build();
    let mut rl = Editor::<()>::with_config(config)?;
    let ud = UserDirs::new().expect("user dirs");
    let mut history = PathBuf::from(ud.home_dir());
    history.push(".psh_history");
    rl.load_history(&history)?;
    let mut prompt_extra = String::from("");
    unsafe { STATE.set(load_state()?).ok() };

    loop {
        let pwd = path_prompt()?;
        let result = rl.readline(&format!("> {} >{} ", pwd, prompt_extra));
        rl.save_history(&history)?;

        let input_line = match result {
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(e) => anyhow::bail!(e),
            Ok(i) => i,
        };

        let command_line = parse_pest(&input_line);
        match command_line {
            Ok(command) => {
                let output = command.run(Stdio::inherit(), Stdio::inherit());

                match output {
                    Ok(output) => {
                        if let Some(mut output) = output.output {
                            let exit = output.wait()?;
                            if exit.success() {
                                prompt_extra = String::from("");
                            } else {
                                let code = exit.code().unwrap();
                                prompt_extra = format!("{}>", code.white().on_red());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    Ok(())
}

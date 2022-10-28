use std::{env, path::PathBuf, process::Stdio};

mod command;
mod parser;

use anyhow::Result;
use directories::UserDirs;
use owo_colors::OwoColorize;
use parser::parse_pest;
use rustyline::Editor;

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

fn main() -> Result<()> {
    let config = rustyline::Config::builder()
        .max_history_size(100)
        .auto_add_history(true)
        .history_ignore_space(true)
        .history_ignore_dups(true)
        .build();
    let mut rl = Editor::<()>::with_config(config)?;
    let ud = UserDirs::new().expect("user dirs");
    let mut history = PathBuf::from(ud.home_dir());
    history.push(".psh_history");
    rl.load_history(&history)?;

    loop {
        let pwd = path_prompt()?;
        let result = rl.readline(&format!("> {} > ", pwd));
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
                            output.wait()?;
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

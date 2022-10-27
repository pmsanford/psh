use std::process::Stdio;

mod command;
mod parser;

use anyhow::Result;
use parser::parse_pest;
use rustyline::Editor;

fn main() -> Result<()> {
    let config = rustyline::Config::builder()
        .max_history_size(100)
        .auto_add_history(true)
        .history_ignore_space(true)
        .history_ignore_dups(true)
        .build();
    let mut rl = Editor::<()>::with_config(config)?;
    rl.load_history(".history")?;

    loop {
        let result = rl.readline(">> ");
        rl.save_history(".history")?;

        let input_line = match result {
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(e) => anyhow::bail!(e),
            Ok(i) => i,
        };

        let command_line = parse_pest(&input_line)?;

        let output = command_line.run(Stdio::inherit(), Stdio::inherit());

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

    Ok(())
}

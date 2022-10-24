use std::process::Stdio;

mod command;
mod parser;

use anyhow::Result;
use parser::parse_line;
use rustyline::Editor;

fn main() -> Result<()> {
    let mut rl = Editor::<()>::new()?;

    loop {
        let result = rl.readline(">> ");

        let input_line = match result {
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(e) => anyhow::bail!(e),
            Ok(i) => i,
        };

        let command_line = parse_line(&input_line)?;

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

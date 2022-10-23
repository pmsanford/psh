use std::{
    env,
    path::Path,
    process::{Child, Command as OsCommand, Stdio},
};

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
        let count = command_line.commands.len();

        let mut prev = None;

        for (idx, cmd) in command_line.commands.into_iter().enumerate() {
            match cmd.command.as_str() {
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
                }
                "exit" => {
                    break;
                }
                "" => {}
                _ => {
                    let last = idx + 1 == count;
                    let stdin = prev.map_or(Stdio::inherit(), |out: Child| {
                        Stdio::from(out.stdout.unwrap())
                    });
                    let stdout = if last {
                        Stdio::inherit()
                    } else {
                        Stdio::piped()
                    };
                    let output = OsCommand::new(cmd.command)
                        .args(cmd.args)
                        .stdin(stdin)
                        .stdout(stdout)
                        .spawn();

                    match (last, output) {
                        (false, Ok(output)) => {
                            prev = Some(output);
                        }
                        (true, Ok(mut output)) => {
                            prev = None;
                            output.wait()?;
                        }
                        (_, Err(e)) => {
                            prev = None;
                            eprintln!("{}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

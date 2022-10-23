use std::process::{exit, Child, Command as OsCommand, Stdio};

mod builtins;
mod parser;

use anyhow::Result;
use builtins::{is_exit, run_builtin};
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
            if is_exit(&cmd)? {
                exit(0);
            }
            if cmd.command.is_empty() {
                continue;
            }
            if run_builtin(&cmd)? {
                continue;
            }
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

    Ok(())
}

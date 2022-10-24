use anyhow::{anyhow, Result};

use crate::command::{Builtin, Command};

pub fn parse_line(input_line: &str) -> Result<Command> {
    let pipelines = input_line.split('|');
    let steps = pipelines
        .map(|text| {
            let mut parts = text.split_whitespace();
            let command = parts.next().unwrap().to_owned();
            let args = parts.map(str::to_owned).collect::<Vec<_>>();

            Ok(match command.as_str() {
                "cd" => Command::Builtin(Builtin::Cd {
                    new_directory: args.first().cloned(),
                }),
                "exit" => Command::Builtin(Builtin::Exit),
                "set" => {
                    let key = args
                        .get(0)
                        .cloned()
                        .ok_or_else(|| anyhow!("Set requires a key and value"))?;
                    let value = args
                        .get(1)
                        .cloned()
                        .ok_or_else(|| anyhow!("Set requires a key and value"))?;
                    Command::Builtin(Builtin::Set { key, value })
                }
                _ => Command::Simple { command, args },
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(Command::Pipeline { steps })
}

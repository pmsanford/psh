use anyhow::Result;
use std::{
    env,
    io::Read,
    path::Path,
    process::{exit, Child, Command as OsCommand, Stdio},
};

use crate::parser::parse_line;

pub enum Builtin {
    Cd { new_directory: Option<String> },
    Set { key: String, value: String },
    Exit,
}

impl Builtin {
    pub fn run(&self, _stdout: Stdio) -> Result<()> {
        match self {
            Builtin::Cd { new_directory } => {
                let new_directory = new_directory
                    .clone()
                    .or_else(|| env::var("HOME").ok())
                    .unwrap_or_else(|| String::from("/"));
                let newpath = Path::new(&new_directory);
                if let Err(e) = env::set_current_dir(newpath) {
                    eprintln!("{}", e);
                }
            }
            Builtin::Set { key, value } => {
                let value = if value.starts_with("$(") {
                    let cmd = value
                        .chars()
                        .skip(2)
                        .take(value.len() - 3)
                        .collect::<String>();
                    let cmd = parse_line(&cmd)?;
                    let stdout = Stdio::piped();
                    let output = cmd.run(Stdio::null(), stdout)?;

                    output
                        .output
                        .and_then(|mut output| output.stdout.take())
                        .map(|mut stdout| {
                            let mut val = String::new();
                            stdout.read_to_string(&mut val)?;
                            Ok::<String, anyhow::Error>(val)
                        })
                        .transpose()?
                        .unwrap_or_default()
                } else {
                    value.to_owned()
                };
                env::set_var(key, value);
            }
            Builtin::Exit => exit(0),
        }

        Ok(())
    }
}

pub enum Command {
    Builtin(Builtin),
    Simple {
        command: String,
        args: Vec<String>,
    },
    Pipeline {
        steps: Vec<Command>,
    },
    And {
        left: Box<Command>,
        right: Box<Command>,
    },
    Or {
        left: Box<Command>,
        right: Box<Command>,
    },
}

pub struct CommandResult {
    pub output: Option<Child>,
}

impl CommandResult {
    pub fn stdout(&mut self) -> Option<Stdio> {
        let mut output = self.output.take();
        let stdio = if let Some(ref mut output) = output {
            output.stdout.take().map(Stdio::from)
        } else {
            None
        };
        self.output = output;
        stdio
    }
}

impl Command {
    pub fn run(&self, stdin: Stdio, stdout: Stdio) -> Result<CommandResult> {
        Ok(match self {
            Command::Builtin(builtin) => {
                builtin.run(stdout)?;
                CommandResult { output: None }
            }
            Command::Simple { command, args } => {
                if command.is_empty() {
                    return Ok(CommandResult { output: None });
                }
                let output = OsCommand::new(command)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;

                CommandResult {
                    output: Some(output),
                }
            }
            Command::Pipeline { steps } => {
                if steps.is_empty() {
                    return Ok(CommandResult { output: None });
                }
                let count = steps.len();
                let mut stdin = stdin;
                for (idx, command) in steps.iter().enumerate() {
                    let end = idx + 1 == count;
                    if end {
                        return command.run(stdin, stdout);
                    }

                    let mut last = command.run(stdin, Stdio::piped())?;

                    stdin = last.stdout().unwrap_or_else(Stdio::null);
                }

                unreachable!()
            }
            Command::And { left, right } => todo!(),
            Command::Or { left, right } => todo!(),
        })
    }
}

use anyhow::Result;
use std::{
    env,
    path::Path,
    process::{exit, Child, Command as OsCommand, Stdio},
};

pub enum Builtin {
    Cd { new_directory: String },
    Exit,
}

impl Builtin {
    pub fn run(&self, stdout: Stdio) -> Result<()> {
        match self {
            Builtin::Cd { new_directory } => {
                let newpath = Path::new(&new_directory);
                if let Err(e) = env::set_current_dir(newpath) {
                    eprintln!("{}", e);
                }
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
    pub stdout: Option<Stdio>,
}

impl CommandResult {
    pub fn stdout(self) -> Stdio {
        if let Some(stdout) = self.stdout {
            return stdout;
        }
        Stdio::from(self.output.unwrap().stdout.unwrap())
    }
}

impl Command {
    pub fn run(&self, stdin: Stdio, stdout: Stdio) -> Result<CommandResult> {
        Ok(match self {
            Command::Builtin(builtin) => {
                builtin.run(stdout)?;
                CommandResult {
                    output: None,
                    stdout: Some(stdin),
                }
            }
            Command::Simple { command, args } => {
                if command.is_empty() {
                    return Ok(CommandResult {
                        output: None,
                        stdout: Some(stdin),
                    });
                }
                let output = OsCommand::new(command)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;

                CommandResult {
                    output: Some(output),
                    stdout: None,
                }
            }
            Command::Pipeline { steps } => {
                if steps.is_empty() {
                    return Ok(CommandResult {
                        output: None,
                        stdout: Some(stdin),
                    });
                }
                let count = steps.len();
                let mut stdin = stdin;
                for (idx, command) in steps.iter().enumerate() {
                    let end = idx + 1 == count;
                    if end {
                        return command.run(stdin, stdout);
                    }

                    let last = command.run(stdin, Stdio::piped())?;

                    stdin = last.stdout();
                }

                unreachable!()
            }
            Command::And { left, right } => todo!(),
            Command::Or { left, right } => todo!(),
        })
    }
}

use anyhow::{bail, Result};
use std::{
    env,
    fs::File,
    path::Path,
    process::{exit, Child, Command as OsCommand, Stdio},
};

use crate::{state::Alias, STATE};

fn run_builtin(command: &Command) -> Result<Option<CommandResult>> {
    Ok(match command {
        Command::Simple { command, args } => match command.as_str() {
            "cd" => {
                let new_directory = args.first();
                let new_directory = new_directory
                    .map(eval_arg)
                    .transpose()?
                    .or_else(|| env::var("HOME").ok())
                    .unwrap_or_else(|| String::from("/"));
                let new_directory = sub_var(&new_directory);
                let newpath = Path::new(&new_directory);
                if let Err(e) = env::set_current_dir(newpath) {
                    eprintln!("{}", e);
                }

                Some(CommandResult { output: None })
            }
            "set" => {
                let mut args = args.iter();
                let key = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Set requires a key and value"))?;
                if let Arg::String { arg_string: key } = key {
                    let value = args
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Set requires a key and value"))?;
                    let value = eval_arg(value)?;
                    env::set_var(key, value.trim());
                } else {
                    bail!("Key must be a string");
                }
                Some(CommandResult { output: None })
            }
            "alias" => {
                if args.len() < 2 {
                    let aliases = unsafe { &STATE.get().unwrap().aliases };
                    for alias in aliases {
                        println!("Alias: {:?}", alias);
                    }
                    return Ok(Some(CommandResult { output: None }));
                }
                let mut args = args.iter();
                let alias = eval_arg(args.next().unwrap())?;
                let command = eval_arg(args.next().unwrap())?;
                let args = args.cloned().collect();

                let aliasdef = Alias {
                    alias: alias.to_owned(),
                    command: command.to_owned(),
                    args,
                };

                unsafe { STATE.get_mut().unwrap().aliases.insert(alias, aliasdef) };

                Some(CommandResult { output: None })
            }
            "exit" => {
                exit(0);
            }
            _ => None,
        },
        _ => None,
    })
}

#[derive(Debug, Clone)]
pub enum Arg {
    String { arg_string: String },
    Env { var_name: String },
    Subcommand { command: Command },
}

#[derive(Debug, Clone)]
pub enum Command {
    Simple {
        command: String,
        args: Vec<Arg>,
    },
    Pipeline {
        steps: Vec<Command>,
        redirect: Option<String>,
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

fn sub_var(arg: &str) -> String {
    if arg.starts_with('$') {
        let key = arg.chars().skip(1).collect::<String>();
        env::var(key).ok().unwrap_or_default()
    } else {
        arg.to_owned()
    }
}

fn eval_arg(arg: &Arg) -> Result<String> {
    Ok(match arg {
        Arg::String { arg_string } => arg_string.clone(),
        Arg::Env { var_name } => env::var(var_name)?,
        Arg::Subcommand { command } => {
            let output = command
                .run(Stdio::null(), Stdio::piped())?
                .output
                .ok_or_else(|| anyhow::anyhow!("Error running subcomand"))?
                .wait_with_output()?;

            if !output.status.success() {
                bail!("Error running subcommand");
            }

            let out_str = String::from_utf8_lossy(&output.stdout).trim().to_owned();

            out_str
        }
    })
}

impl Command {
    pub fn run(&self, stdin: Stdio, stdout: Stdio) -> Result<CommandResult> {
        Ok(match self {
            Command::Simple { command, args } => {
                if command.is_empty() {
                    return Ok(CommandResult { output: None });
                }
                if let Some(result) = run_builtin(self)? {
                    return Ok(result);
                }
                let (command, args) =
                    if let Some(alias) = unsafe { STATE.get().unwrap().aliases.get(command) } {
                        let mut merged_args = alias.args.clone();
                        merged_args.append(&mut args.clone());
                        (alias.command.clone(), merged_args)
                    } else {
                        (command.clone(), args.clone())
                    };
                let args = args.iter().map(eval_arg).collect::<Result<Vec<_>>>()?;
                let output = OsCommand::new(command)
                    .args(args)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;

                CommandResult {
                    output: Some(output),
                }
            }
            Command::Pipeline { steps, redirect } => {
                if steps.is_empty() {
                    return Ok(CommandResult { output: None });
                }
                let count = steps.len();
                let mut stdin = stdin;
                for (idx, command) in steps.iter().enumerate() {
                    let end = idx + 1 == count;
                    if end {
                        if let Some(redirect) = redirect {
                            let file = File::create(redirect)?;
                            let fileout = Stdio::from(file);
                            return command.run(stdin, fileout);
                        }
                        return command.run(stdin, stdout);
                    }

                    let mut last = command.run(stdin, Stdio::piped())?;

                    stdin = last.stdout().unwrap_or_else(Stdio::null);
                }

                unreachable!()
            }
            Command::And { left, right } => {
                let lresult = left.run(stdin, Stdio::inherit())?;
                let mut output = lresult.output.unwrap();

                if !output.wait()?.success() {
                    return Ok(CommandResult {
                        output: Some(output),
                    });
                }

                right.run(Stdio::null(), Stdio::inherit())?
            }
            Command::Or { left, right } => {
                let lresult = left.run(stdin, Stdio::inherit())?;
                let mut output = lresult.output.unwrap();

                if output.wait()?.success() {
                    CommandResult {
                        output: Some(output),
                    }
                } else {
                    right.run(Stdio::null(), Stdio::inherit())?
                }
            }
        })
    }
}

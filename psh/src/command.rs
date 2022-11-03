use anyhow::{bail, Result};
use async_recursion::async_recursion;
use protos::{
    create_channel, env_client::EnvClient, sock_path_from_pid, status_client::StatusClient,
};
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
    process::{exit, Child, Command as OsCommand, Stdio},
    sync::{Arc, Mutex},
};
use tonic::Request;

use crate::state::{Alias, State};

async fn run_builtin(
    command: &Command,
    state: &Arc<Mutex<State>>,
) -> Result<Option<CommandResult>> {
    Ok(match command {
        Command::Simple { command, args } => match command.as_str() {
            "copyenv" => {
                if args.len() != 2 {
                    eprintln!("copyenv takes a pid and a var name");
                    return Ok(Some(CommandResult { output: None }));
                }
                let mut args = args.iter();
                let arg = args.next().unwrap();
                let pid = eval_arg(&arg, &state).await?;
                let pid: u32 = pid.parse()?;
                let key = args.next().unwrap();
                let key = eval_arg(&key, &state).await?;

                let sock_path = sock_path_from_pid(pid);

                let channel = create_channel(sock_path).await?;

                let mut client = EnvClient::new(channel);

                let request = Request::new(());

                let resp = client.get_env(request).await?.into_inner();

                let vars = resp
                    .vars
                    .into_iter()
                    .map(|var| (var.key, var.value))
                    .collect::<HashMap<_, _>>();

                std::env::set_var(
                    key.clone(),
                    vars.get(&key).map(|v| v.clone()).unwrap_or_default(),
                );

                Some(CommandResult { output: None })
            }
            "pshl" => {
                if args.len() != 0 {
                    eprintln!("pshl doesn't take any args");
                    return Ok(Some(CommandResult { output: None }));
                }

                let psh_path = PathBuf::from("/tmp/psh");

                for dir in std::fs::read_dir(psh_path)? {
                    if let Ok(dir) = dir {
                        if let Ok(pid) = dir.file_name().to_string_lossy().parse::<u32>() {
                            if pid == std::process::id() {
                                continue;
                            }
                            let sock_path = sock_path_from_pid(pid);
                            if let Ok(channel) = create_channel(sock_path).await {
                                let mut client = StatusClient::new(channel);

                                let request = Request::new(());

                                if let Ok(resp) = client.get_status(request).await {
                                    let resp = resp.into_inner();
                                    println!(
                                        "{}: {} ({})",
                                        dir.file_name().to_string_lossy(),
                                        resp.current_command,
                                        resp.working_dir
                                    );
                                }
                            }
                        }
                    }
                }

                Some(CommandResult { output: None })
            }
            "diffenv" => {
                if args.len() != 1 {
                    eprintln!("diffenv requires a process id");
                    return Ok(Some(CommandResult { output: None }));
                }

                let arg = args.iter().next().unwrap();
                let pid = eval_arg(&arg, &state).await?;
                let pid: u32 = pid.parse()?;

                let sock_path = sock_path_from_pid(pid);

                let channel = create_channel(sock_path).await?;

                let mut client = EnvClient::new(channel);

                let request = Request::new(());

                let resp = client.get_env(request).await?.into_inner();

                let local_vars = std::env::vars().into_iter().collect::<HashMap<_, _>>();

                for var in resp.vars {
                    let local = local_vars.get(&var.key);

                    if let Some(local) = local {
                        if *local != var.value {
                            println!(" {}:", var.key);
                            println!("\tLocal:  {}", local);
                            println!("\tTheirs: {}", var.value);
                        }
                    } else {
                        println!("+{}: {}", var.key, var.value);
                    }
                }

                Some(CommandResult { output: None })
            }
            "cd" => {
                let new_directory = args.first();
                let new_directory = if let Some(arg) = new_directory {
                    Some(eval_arg(arg, state).await?)
                } else {
                    env::var("HOME").ok()
                }
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
                    let value = eval_arg(value, state).await?;
                    env::set_var(key, value.trim());
                } else {
                    bail!("Key must be a string");
                }
                Some(CommandResult { output: None })
            }
            "alias" => {
                if args.len() < 2 {
                    let aliases = state.lock().unwrap().aliases.clone();
                    for (_, alias) in aliases.iter() {
                        println!("{}", alias.display());
                    }
                    return Ok(Some(CommandResult { output: None }));
                }
                let mut args = args.iter();
                let alias = eval_arg(args.next().unwrap(), state).await?;
                let command = eval_arg(args.next().unwrap(), state).await?;
                let args = args.cloned().collect();

                let aliasdef = Alias {
                    alias: alias.to_owned(),
                    command,
                    args,
                };

                state.lock().unwrap().aliases.insert(alias, aliasdef);

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

impl Display for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arg::String { arg_string } => arg_string.fmt(f),
            Arg::Env { var_name } => var_name.fmt(f),
            Arg::Subcommand { command } => f.write_fmt(format_args!("{:?}", command)),
        }
    }
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

async fn eval_arg(arg: &Arg, state: &Arc<Mutex<State>>) -> Result<String> {
    Ok(match arg {
        Arg::String { arg_string } => arg_string.clone(),
        Arg::Env { var_name } => env::var(var_name)?,
        Arg::Subcommand { command } => {
            let output = command
                .run(Stdio::null(), Stdio::piped(), state)
                .await?
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
    #[async_recursion]
    pub async fn run(
        &self,
        stdin: Stdio,
        stdout: Stdio,
        state: &Arc<Mutex<State>>,
    ) -> Result<CommandResult> {
        Ok(match self {
            Command::Simple { command, args } => {
                if command.is_empty() {
                    return Ok(CommandResult { output: None });
                }
                if let Some(result) = run_builtin(self, state).await? {
                    return Ok(result);
                }
                let (command, args) =
                    if let Some(alias) = state.lock().unwrap().aliases.get(command) {
                        let mut merged_args = alias.args.clone();
                        merged_args.append(&mut args.clone());
                        (alias.command.clone(), merged_args)
                    } else {
                        (command.clone(), args.clone())
                    };
                let mut arg_vec = vec![];
                for arg in args {
                    arg_vec.push(eval_arg(&arg, state).await?);
                }
                state.lock().unwrap().current_command = Some(command.clone());
                let output = OsCommand::new(command)
                    .args(arg_vec)
                    .stdin(stdin)
                    .stdout(stdout)
                    .spawn()?;
                state.lock().unwrap().running_pid = Some(output.id());

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
                            return command.run(stdin, fileout, state).await;
                        }
                        return command.run(stdin, stdout, state).await;
                    }

                    let mut last = command.run(stdin, Stdio::piped(), state).await?;

                    stdin = last.stdout().unwrap_or_else(Stdio::null);
                }

                unreachable!()
            }
            Command::And { left, right } => {
                let lresult = left.run(stdin, Stdio::inherit(), state).await?;
                let mut output = lresult.output.unwrap();

                if !output.wait()?.success() {
                    return Ok(CommandResult {
                        output: Some(output),
                    });
                }

                right.run(Stdio::null(), Stdio::inherit(), state).await?
            }
            Command::Or { left, right } => {
                let lresult = left.run(stdin, Stdio::inherit(), state).await?;
                let mut output = lresult.output.unwrap();

                if output.wait()?.success() {
                    CommandResult {
                        output: Some(output),
                    }
                } else {
                    right.run(Stdio::null(), Stdio::inherit(), state).await?
                }
            }
        })
    }
}

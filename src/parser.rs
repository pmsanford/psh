use anyhow::{anyhow, Result};
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::command::{Builtin, Command};

#[derive(Parser)]
#[grammar = "cli.pest"]
struct CliParser;

pub fn parse_pest(input_line: &str) -> Result<Command> {
    let parsed = CliParser::parse(Rule::pipeline, input_line)?;

    let pipe = parsed.into_iter().next().unwrap();

    recurse_commands(pipe)
}

pub fn recurse_commands(pair: Pair<Rule>) -> Result<Command> {
    match pair.as_rule() {
        Rule::special => unreachable!(),
        Rule::charsa => unreachable!(),
        Rule::chars => unreachable!(),
        Rule::litchars => unreachable!(),
        Rule::WHITESPACE => unreachable!(),
        Rule::literal => unreachable!(),
        Rule::word => unreachable!(),
        Rule::var => unreachable!(),
        Rule::subcmd => todo!(),
        Rule::invocation => {
            let pairs = pair.into_inner().collect::<Vec<_>>();
            let cmd = pairs
                .iter()
                .find(|p| p.as_rule() == Rule::command)
                .cloned()
                .unwrap();
            let args = pairs
                .iter()
                .filter(|p| p.as_rule() == Rule::arg)
                .cloned()
                .collect::<Vec<_>>();
            Ok(Command::Simple {
                command: cmd.as_str().to_owned(),
                args: args
                    .into_iter()
                    .map(|arg| arg.as_str().to_owned())
                    .collect(),
            })
        }
        Rule::arg => unreachable!(),
        Rule::command => unreachable!(),
        Rule::binop => unreachable!(),
        Rule::bin => todo!(),
        Rule::pipeline => {
            let mut steps = vec![];
            for child in pair.into_inner() {
                steps.push(recurse_commands(child)?);
            }

            Ok(Command::Pipeline { steps })
        }
    }
}

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

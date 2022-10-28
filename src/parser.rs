use anyhow::Result;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::command::{Arg, Command};

#[derive(Parser)]
#[grammar = "cli.pest"]
struct CliParser;

pub fn parse_pest(input_line: &str) -> Result<Command> {
    let parsed = CliParser::parse(Rule::bin, input_line)?;
    let parsed_len = parsed.as_str().len();
    if parsed_len < input_line.len() {
        anyhow::bail!("Syntax error");
    }

    let pipe = parsed.into_iter().next().unwrap();

    recurse_commands(pipe)
}

fn recurse_args(pair: Pair<Rule>) -> Result<Arg> {
    let pair = pair.into_inner().into_iter().next().unwrap();
    Ok(match pair.as_rule() {
        Rule::subcmd => {
            let pipe = pair.into_inner().into_iter().next().unwrap();
            let pipe = recurse_commands(pipe)?;
            Arg::Subcommand { command: pipe }
        }
        Rule::var => Arg::Env {
            var_name: pair
                .into_inner()
                .into_iter()
                .next()
                .unwrap()
                .as_str()
                .to_owned(),
        },
        Rule::literal => Arg::String {
            arg_string: pair
                .into_inner()
                .into_iter()
                .next()
                .unwrap()
                .as_str()
                .to_owned(),
        },
        Rule::word => Arg::String {
            arg_string: pair.as_str().to_owned(),
        },
        _ => unreachable!(),
    })
}

pub fn recurse_commands(pair: Pair<Rule>) -> Result<Command> {
    match pair.as_rule() {
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
                .map(recurse_args)
                .collect::<Result<Vec<_>>>()?;
            Ok(Command::Simple {
                command: cmd.as_str().to_owned(),
                args,
            })
        }
        Rule::bin => {
            let mut pairs = pair.into_inner().collect::<Vec<_>>();
            assert!(pairs.len() % 2 == 1);
            let right = pairs.pop().unwrap();
            let mut right = recurse_commands(right)?;
            pairs.reverse();

            for chunk in pairs.chunks(2) {
                let chunk = chunk.to_vec();
                let op = chunk.get(0).unwrap().clone();
                let left = chunk.get(1).unwrap().clone();
                let left = recurse_commands(left)?;
                let op = op.into_inner().into_iter().next().unwrap();
                right = match op.as_rule() {
                    Rule::and => Command::And {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    Rule::or => Command::Or {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    _ => unreachable!(),
                };
            }

            Ok(right)
        }
        Rule::pipeline => {
            let mut steps = vec![];
            for child in pair.into_inner() {
                steps.push(recurse_commands(child)?);
            }

            Ok(Command::Pipeline { steps })
        }
        _ => unreachable!(),
    }
}

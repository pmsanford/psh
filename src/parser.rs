use anyhow::Result;

pub struct Command {
    pub command: String,
    pub args: Vec<String>,
}

pub struct CommandLine {
    pub commands: Vec<Command>,
}

pub fn parse_line(input_line: &str) -> Result<CommandLine> {
    let pipelines = input_line.split('|');
    let commands = pipelines
        .map(|text| {
            let mut parts = text.split_whitespace();
            let command = parts.next().unwrap().to_owned();
            let args = parts.map(str::to_owned).collect::<Vec<_>>();

            Command { command, args }
        })
        .collect::<Vec<_>>();

    Ok(CommandLine { commands })
}

use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::Result;
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

        let mut parts = input_line.split_whitespace();
        let cmd = parts.next().unwrap();
        let args = parts.collect::<Vec<_>>();

        match cmd {
            "cd" => {
                let newpath = args
                    .first()
                    .map(|s| String::from(*s))
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
                let output = Command::new(cmd)
                    .args(args)
                    .stdout(Stdio::inherit())
                    .spawn();

                match output {
                    Ok(mut output) => {
                        output.wait()?;
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

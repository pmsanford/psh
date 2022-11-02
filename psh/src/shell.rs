use std::{
    collections::HashMap,
    env,
    fs::File,
    io::Read,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use directories::{ProjectDirs, UserDirs};
use owo_colors::OwoColorize;
use rustyline::{
    completion::FilenameCompleter, highlight::Highlighter, hint::HistoryHinter, CompletionType,
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};

use crate::{parser::parse_pest, state::State};

fn path_prompt() -> Result<String> {
    let ud = UserDirs::new().ok_or_else(|| anyhow::anyhow!("Couldn't find user dirs"))?;
    let home = ud.home_dir();

    let pwd = env::current_dir()?;

    if pwd.starts_with(home) {
        Ok(format!(
            "~/{}",
            pwd.strip_prefix(home)?.to_string_lossy().yellow()
        ))
    } else {
        Ok(format!(
            "{}",
            pwd.to_string_lossy().to_string().bright_blue()
        ))
    }
}

async fn load_state() -> Result<Arc<Mutex<State>>> {
    let ud = UserDirs::new().expect("user dirs");
    let mut history_path = PathBuf::from(ud.home_dir());
    history_path.push(".psh_history");
    let state = Arc::new(Mutex::new(State {
        aliases: HashMap::new(),
        history_path,
        current_command: None,
    }));

    run_rc(&state).await?;

    Ok(state)
}

async fn run_rc(state: &Arc<Mutex<State>>) -> Result<()> {
    let pd = ProjectDirs::from("net", "paulsanford", "psh").expect("project dirs");
    let mut rc = PathBuf::from(pd.config_dir());
    rc.push("pshrc");

    if !rc.exists() {
        return Ok(());
    }

    let mut file = File::open(rc)?;
    let mut cont = String::new();
    file.read_to_string(&mut cont)?;

    for (num, line) in cont.lines().enumerate() {
        match parse_pest(line) {
            Ok(parsed) => {
                if let Err(e) = parsed.run(Stdio::null(), Stdio::inherit(), &state).await {
                    eprintln!("Error on line {} of rc file: {}", num, e);
                }
            }
            Err(e) => {
                eprintln!("Error on line {} of rc file: {}", num, e);
            }
        }
    }

    Ok(())
}

#[derive(Completer, Helper, Validator, Highlighter, Hinter)]
struct PshHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    #[rustyline(Highlighter)]
    highlighter: PshHighlighter,
}

struct PshHighlighter;

impl Highlighter for PshHighlighter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(format!("{}", hint.truecolor(75, 75, 75)))
    }
}

pub struct Pshell {
    state: Arc<Mutex<State>>,
    editor: Editor<PshHelper>,
}

impl Pshell {
    pub fn get_state_ref(&self) -> Arc<Mutex<State>> {
        Arc::clone(&self.state)
    }

    pub async fn new() -> Result<Self> {
        let state = load_state().await?;
        let config = rustyline::Config::builder()
            .max_history_size(100)
            .auto_add_history(true)
            .history_ignore_space(true)
            .history_ignore_dups(true)
            .completion_type(CompletionType::List)
            .build();
        let h = PshHelper {
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter {},
            highlighter: PshHighlighter,
        };
        let mut editor = Editor::<PshHelper>::with_config(config)?;
        editor.set_helper(Some(h));
        editor.load_history(&state.lock().unwrap().history_path)?;

        Ok(Self { state, editor })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut prompt_extra = String::from("");

        loop {
            let pwd = path_prompt()?;
            let result = self
                .editor
                .readline(&format!("> {} >{} ", pwd, prompt_extra));
            {
                let state = self.state.lock().unwrap();
                self.editor.save_history(&state.history_path)?;
            }

            let input_line = match result {
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(e) => anyhow::bail!(e),
                Ok(i) => i,
            };

            let command_line = parse_pest(&input_line);
            match command_line {
                Ok(command) => {
                    let output = command
                        .run(Stdio::inherit(), Stdio::inherit(), &self.state)
                        .await;

                    match output {
                        Ok(output) => {
                            if let Some(mut output) = output.output {
                                let exit = output.wait()?;
                                self.state.lock().unwrap().current_command = None;
                                if exit.success() {
                                    prompt_extra = String::from("");
                                } else {
                                    let code = exit.code().unwrap();
                                    prompt_extra = format!("{}>", code.white().on_red());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        }

        Ok(())
    }
}

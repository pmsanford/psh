use std::{collections::HashMap, env, fs::File, io::Read, path::PathBuf, process::Stdio};

use anyhow::Result;
use directories::{ProjectDirs, UserDirs};
use owo_colors::OwoColorize;
use rustyline::{
    completion::FilenameCompleter, highlight::Highlighter, hint::HistoryHinter, CompletionType,
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};

use crate::{
    parser::{parse_alias, parse_pest},
    state::{Alias, State},
};

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

fn load_state() -> Result<State> {
    let aliases = load_aliases()?;
    let ud = UserDirs::new().expect("user dirs");
    let mut history_path = PathBuf::from(ud.home_dir());
    history_path.push(".psh_history");
    Ok(State {
        aliases,
        history_path,
    })
}

fn load_aliases() -> Result<HashMap<String, Alias>> {
    let pd = ProjectDirs::from("net", "paulsanford", "psh").expect("project dirs");
    let mut aliases = PathBuf::from(pd.config_dir());
    aliases.push("aliases");

    if !aliases.exists() {
        return Ok(HashMap::new());
    }

    let mut file = File::open(aliases)?;
    let mut cont = String::new();
    file.read_to_string(&mut cont)?;

    let mut hm = HashMap::new();

    for line in cont.lines() {
        let alias = parse_alias(line)?;
        hm.insert(alias.alias.clone(), alias);
    }

    Ok(hm)
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
    state: State,
    editor: Editor<PshHelper>,
}

impl Pshell {
    pub fn new() -> Result<Self> {
        let state = load_state()?;
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
        editor.load_history(&state.history_path)?;

        Ok(Self { state, editor })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut prompt_extra = String::from("");

        loop {
            let pwd = path_prompt()?;
            let result = self
                .editor
                .readline(&format!("> {} >{} ", pwd, prompt_extra));
            self.editor.save_history(&self.state.history_path)?;

            let input_line = match result {
                Err(rustyline::error::ReadlineError::Eof) => break,
                Err(e) => anyhow::bail!(e),
                Ok(i) => i,
            };

            let command_line = parse_pest(&input_line);
            match command_line {
                Ok(command) => {
                    let output = command.run(Stdio::inherit(), Stdio::inherit(), &mut self.state);

                    match output {
                        Ok(output) => {
                            if let Some(mut output) = output.output {
                                let exit = output.wait()?;
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

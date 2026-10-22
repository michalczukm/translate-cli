use crate::languages::{normalize_source, normalize_target};
use crate::{config, deepl, dictionary, prompt, render, variants};

pub const USAGE: &str = "\
translate - DeepL from the terminal

Usage:
    translate [--plain] <FROM:TO> [TEXT...]   pl:en, :de (auto-detect source)
    translate usage                           characters used this month
    translate auth set-key [KEY]              store an API key
    translate auth path                       where the key lives
    translate setup                           shell shortcuts (tpl/ten or tde/ten) and API key

TEXT is read from stdin when omitted.

Options:
    --plain          translation and alternative only, no decoration or color
    --synonyms       single English words: compact synonyms plus antonyms
    -h, --help       show this help
    -V, --version    show the version
";

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    pub pair: String,
    pub text: Vec<String>,
    pub plain: bool,
    pub synonyms: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Args {
    Run(Cli),
    Message(String),
}

/// Everything after the language pair is text, so flags only count before it.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut plain = false;
    let mut synonyms = false;
    let mut rest = Vec::new();
    let mut args = args.into_iter();

    for arg in args.by_ref() {
        match arg.as_str() {
            "--plain" => plain = true,
            "--synonyms" => synonyms = true,
            "-h" | "--help" => return Ok(Args::Message(USAGE.to_string())),
            "-V" | "--version" => {
                return Ok(Args::Message(format!(
                    "translate {}\n",
                    env!("CARGO_PKG_VERSION")
                )));
            }
            _ => {
                rest.push(arg);
                break;
            }
        }
    }

    let mut rest = rest.into_iter().chain(args);
    let pair = rest.next().ok_or_else(|| USAGE.to_string())?;
    Ok(Args::Run(Cli {
        pair,
        text: rest.collect(),
        plain,
        synonyms,
    }))
}

pub const KEY_HELP: &str = "\
translate uses DeepL to translate, which needs an API key.
The free tier (500,000 characters/month) is plenty for home use.
Get one: https://www.deepl.com/pro-api
Already have an account? Your keys: https://www.deepl.com/your-account/keys";

pub fn auth_command(args: &[String]) -> Result<String, (i32, String)> {
    match args.first().map(String::as_str) {
        Some("set-key") => {
            let key = match args.get(1) {
                Some(k) => k.clone(),
                None => {
                    println!("{KEY_HELP}");
                    let typed = prompt::read_secret("DeepL API key: ").map_err(|m| (2, m))?;
                    if typed.is_empty() {
                        return Err((2, "no key entered".to_string()));
                    }
                    typed
                }
            };
            let path = config::save(&key).map_err(|m| (1, m))?;
            Ok(format!("key saved to {} (0600)\n", tilde(&path)))
        }
        Some("path") => {
            let path = config::config_path().ok_or((1, "HOME is not set".to_string()))?;
            Ok(format!("{}\n", path.display()))
        }
        Some(other) => Err((
            2,
            format!("unknown auth subcommand `{other}` (try set-key)"),
        )),
        None => Err((2, "auth needs a subcommand (set-key or path)".to_string())),
    }
}

const SHORTCUTS_MARKER: &str = "# translate-cli shortcuts";

fn shell_functions(shell: &str, lang: &str) -> String {
    if shell == "fish" {
        return format!(
            "\n{SHORTCUTS_MARKER}\n\
             function t{lang}; translate en:{lang} $argv; end\n\
             function ten; translate {lang}:en $argv; end\n"
        );
    }
    format!(
        "\n{SHORTCUTS_MARKER}\n\
         t{lang}() {{ translate en:{lang} \"$@\"; }}\n\
         ten() {{ translate {lang}:en \"$@\"; }}\n"
    )
}

fn parse_language(input: &str) -> Option<&'static str> {
    match input.trim().to_lowercase().as_str() {
        "pl" => Some("pl"),
        "de" => Some("de"),
        _ => None,
    }
}

/// macOS terminals open login shells, which read ~/.bash_profile and skip ~/.bashrc.
fn rc_file(shell: &str, macos: bool) -> Option<&'static str> {
    match shell {
        "zsh" => Some(".zshrc"),
        "bash" if macos => Some(".bash_profile"),
        "bash" => Some(".bashrc"),
        "fish" => Some(".config/fish/conf.d/translate-cli.fish"),
        _ => None,
    }
}

/// Talks on stderr: on a first run stdout carries the translation that follows.
pub fn setup_command() -> Result<String, (i32, String)> {
    setup_shortcuts()?;
    setup_key()?;
    Ok(String::new())
}

fn setup_shortcuts() -> Result<(), (i32, String)> {
    let shell_path = std::env::var("SHELL").unwrap_or_default();
    let shell = shell_path.rsplit('/').next().unwrap_or_default();
    let Some(rc) = rc_file(shell, cfg!(target_os = "macos")) else {
        let lang = ask_language()?;
        eprintln!(
            "unsupported shell `{shell_path}`, add these yourself:\n{}",
            shell_functions("bash", lang)
        );
        return Ok(());
    };
    let home = std::env::var_os("HOME").ok_or((1, "HOME is not set".to_string()))?;
    let path = std::path::PathBuf::from(home).join(rc);
    if has_shortcuts(&path).map_err(|e| (1, e.to_string()))? {
        eprintln!("shortcuts already in {}", tilde(&path));
        return Ok(());
    }
    let lang = ask_language()?;
    append_shortcuts(&path, shell, lang).map_err(|e| (1, e.to_string()))?;
    eprintln!(
        "added t{lang} and ten to {p}, run: source {p}",
        p = tilde(&path)
    );
    Ok(())
}

fn ask_language() -> Result<&'static str, (i32, String)> {
    loop {
        let typed = prompt::read_line("Which language do you translate with English? [pl/de]: ")
            .map_err(|m| (2, m))?;
        match parse_language(&typed) {
            Some(lang) => return Ok(lang),
            None => eprintln!("pick pl or de"),
        }
    }
}

fn setup_key() -> Result<(), (i32, String)> {
    if config::load().is_ok() {
        eprintln!("DeepL API key already set");
        return Ok(());
    }
    eprintln!("{KEY_HELP}");
    let key = prompt::read_secret("DeepL API key: ").map_err(|m| (2, m))?;
    if key.is_empty() {
        eprintln!("no key entered, run `translate auth set-key` later");
        return Ok(());
    }
    let path = config::save(&key).map_err(|m| (1, m))?;
    eprintln!("key saved to {} (0600)", tilde(&path));
    Ok(())
}

fn tilde(path: &std::path::Path) -> String {
    let home = std::env::var_os("HOME").unwrap_or_default();
    match path.strip_prefix(&home) {
        Ok(rest) if !home.is_empty() => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

fn has_shortcuts(path: &std::path::Path) -> std::io::Result<bool> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(s.contains(SHORTCUTS_MARKER)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

fn append_shortcuts(path: &std::path::Path, shell: &str, lang: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?
        .write_all(shell_functions(shell, lang).as_bytes())
}

pub fn parse_pair(pair: &str) -> Result<(Option<String>, String), String> {
    let (from, to) = match pair.split_once(':') {
        Some((f, t)) => (f, t),
        None => ("", pair),
    };
    if to.trim().is_empty() {
        return Err(format!(
            "missing target language in `{pair}` (expected FROM:TO, e.g. pl:en)"
        ));
    }
    let source = (!from.trim().is_empty()).then(|| normalize_source(from));
    Ok((source, normalize_target(to)))
}

/// `usage` takes no text, and a terminal on stdin means nobody is piping any:
/// reading it would just hang.
pub fn needs_stdin(pair: &str, text: &[String], stdin_is_terminal: bool) -> bool {
    text.is_empty() && pair != "usage" && !stdin_is_terminal
}

pub fn text_from(args: &[String], stdin: &str) -> String {
    if args.is_empty() {
        stdin.trim().to_string()
    } else {
        args.join(" ")
    }
}

pub fn run(cli: Cli, key: String, color: bool, stdin: &str) -> Result<String, (i32, String)> {
    let client = deepl::Client::new(key);

    if cli.pair == "usage" {
        let u = client.usage().map_err(|e| (1, e.to_string()))?;
        let pct = if u.character_limit == 0 {
            0.0
        } else {
            u.character_count as f64 * 100.0 / u.character_limit as f64
        };
        return Ok(format!(
            "{} / {} characters ({:.1}%)\n",
            u.character_count, u.character_limit, pct
        ));
    }

    let (source, target) = parse_pair(&cli.pair).map_err(|m| (2, m))?;
    let text = text_from(&cli.text, stdin);
    if text.is_empty() {
        return Err((
            2,
            "no text given (pass it as an argument or on stdin)".into(),
        ));
    }

    let did_you_mean = variants::fix_source(&client, &text, source.as_deref());
    let to_translate = did_you_mean.as_deref().unwrap_or(&text);

    let translated = client
        .translate(to_translate, source.as_deref(), &target, None)
        .map_err(|e| (1, e.to_string()))?;

    // Datamuse only covers English, and only single words carry word senses.
    // --plain never renders definitions, so don't pay for the lookup.
    let definitions = (!cli.plain && normalize_source(&target) == "EN")
        .then(|| dictionary::lookup(&translated.text))
        .flatten();

    let antonyms = match &definitions {
        Some(entry) if cli.synonyms => {
            dictionary::antonyms(&translated.text, &entry.part_of_speech)
        }
        _ => Vec::new(),
    };

    let alternative = match definitions {
        Some(_) => None,
        None => variants::alternative(
            &client,
            to_translate,
            source.as_deref(),
            &target,
            &translated.text,
        ),
    };

    let out = render::Output {
        source_lang: source.unwrap_or(translated.detected_source_language),
        target_lang: target,
        source_text: text,
        did_you_mean,
        translation: translated.text,
        alternative,
        definitions,
        synonyms: cli.synonyms,
        antonyms,
    };

    Ok(if cli.plain {
        render::plain(&out)
    } else {
        render::full(&out, color)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(raw: &[&str]) -> Result<Args, String> {
        parse_args(raw.iter().map(|s| s.to_string()))
    }

    #[test]
    fn flags_before_the_pair_are_parsed_the_rest_is_text() {
        assert_eq!(
            args(&["--plain", "--synonyms", "pl:en", "dzień", "dobry"]).unwrap(),
            Args::Run(Cli {
                pair: "pl:en".into(),
                text: vec!["dzień".into(), "dobry".into()],
                plain: true,
                synonyms: true,
            })
        );
    }

    #[test]
    fn flags_after_the_pair_are_text() {
        let Ok(Args::Run(cli)) = args(&["pl:en", "--plain"]) else {
            panic!("expected a run")
        };
        assert!(!cli.plain);
        assert_eq!(cli.text, vec!["--plain".to_string()]);
    }

    #[test]
    fn shortcuts_are_detected_after_appending() {
        let path = std::env::temp_dir().join(format!("translate-rc-{}", std::process::id()));
        std::fs::write(&path, "export FOO=1\n").unwrap();
        assert!(!has_shortcuts(&path).unwrap());
        append_shortcuts(&path, "zsh", "de").unwrap();
        assert!(has_shortcuts(&path).unwrap());
        let rc = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            rc,
            format!("export FOO=1\n{}", shell_functions("zsh", "de"))
        );
    }

    #[test]
    fn shortcuts_pair_the_language_with_english() {
        let pl = shell_functions("zsh", "pl");
        assert!(pl.contains("tpl() { translate en:pl \"$@\"; }"));
        assert!(pl.contains("ten() { translate pl:en \"$@\"; }"));
        let de = shell_functions("bash", "de");
        assert!(de.contains("tde() { translate en:de \"$@\"; }"));
        assert!(de.contains("ten() { translate de:en \"$@\"; }"));
        let fish = shell_functions("fish", "de");
        assert!(fish.contains("function tde; translate en:de $argv; end"));
        assert!(fish.contains("function ten; translate de:en $argv; end"));
    }

    #[test]
    fn rc_file_per_shell() {
        assert_eq!(rc_file("zsh", true), Some(".zshrc"));
        assert_eq!(rc_file("bash", true), Some(".bash_profile"));
        assert_eq!(rc_file("bash", false), Some(".bashrc"));
        assert_eq!(
            rc_file("fish", false),
            Some(".config/fish/conf.d/translate-cli.fish")
        );
        assert_eq!(rc_file("nu", true), None);
    }

    #[test]
    fn paths_under_home_print_with_a_tilde() {
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        assert_eq!(tilde(&home.join(".zshrc")), "~/.zshrc");
        assert_eq!(tilde(std::path::Path::new("/etc/zshrc")), "/etc/zshrc");
    }

    #[test]
    fn only_polish_and_german_are_offered() {
        assert_eq!(parse_language(" DE\n"), Some("de"));
        assert_eq!(parse_language("pl"), Some("pl"));
        assert_eq!(parse_language("fr"), None);
        assert_eq!(parse_language(""), None);
    }

    #[test]
    fn help_and_version_short_circuit() {
        assert_eq!(args(&["--help"]).unwrap(), Args::Message(USAGE.to_string()));
        let Ok(Args::Message(v)) = args(&["-V"]) else {
            panic!("expected a message")
        };
        assert_eq!(v, format!("translate {}\n", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn no_arguments_is_an_error_carrying_the_usage() {
        assert_eq!(args(&[]).unwrap_err(), USAGE);
    }

    #[test]
    fn auth_requires_a_known_subcommand() {
        assert_eq!(auth_command(&[]).unwrap_err().0, 2);
        assert_eq!(auth_command(&["nope".to_string()]).unwrap_err().0, 2);
    }

    #[test]
    fn auth_path_prints_the_config_location() {
        let out = auth_command(&["path".to_string()]).unwrap();
        assert!(out.trim().ends_with(".config/translate-cli/.env"));
    }

    #[test]
    fn pair_with_both_sides() {
        assert_eq!(
            parse_pair("pl:en").unwrap(),
            (Some("PL".into()), "EN-US".into())
        );
        assert_eq!(
            parse_pair("EN:de").unwrap(),
            (Some("EN".into()), "DE".into())
        );
    }

    #[test]
    fn pair_with_auto_detected_source() {
        assert_eq!(parse_pair(":de").unwrap(), (None, "DE".into()));
    }

    #[test]
    fn bare_target_is_auto_detected_source() {
        assert_eq!(parse_pair("de").unwrap(), (None, "DE".into()));
    }

    #[test]
    fn empty_target_is_rejected() {
        assert!(parse_pair("pl:").is_err());
        assert!(parse_pair("").is_err());
    }

    #[test]
    fn stdin_is_read_only_when_it_could_carry_text() {
        assert!(needs_stdin("pl:en", &[], false));
        assert!(!needs_stdin("pl:en", &[], true));
        assert!(!needs_stdin("usage", &[], false));
        assert!(!needs_stdin("pl:en", &["hi".to_string()], false));
    }

    #[test]
    fn args_join_with_spaces_and_stdin_is_the_fallback() {
        let args = vec!["dzień".to_string(), "dobry".to_string()];
        assert_eq!(text_from(&args, "ignored"), "dzień dobry");
        assert_eq!(text_from(&[], "  from stdin\n"), "from stdin");
    }
}

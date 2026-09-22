use crate::languages::{normalize_source, normalize_target};
use crate::{config, deepl, dictionary, prompt, render, variants};

pub const USAGE: &str = "\
translate - DeepL from the terminal

Usage:
    translate [--plain] <FROM:TO> [TEXT...]   pl:en, :de (auto-detect source)
    translate usage                           characters used this month
    translate auth set-key [KEY]              store an API key
    translate auth path                       where the key lives

TEXT is read from stdin when omitted.

Options:
    --plain          translation and alternative only, no decoration or color
    -h, --help       show this help
    -V, --version    show the version
";

#[derive(Debug, PartialEq, Eq)]
pub struct Cli {
    pub pair: String,
    pub text: Vec<String>,
    pub plain: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Args {
    Run(Cli),
    Message(String),
}

/// Everything after the language pair is text, so flags only count before it.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut plain = false;
    let mut rest = Vec::new();
    let mut args = args.into_iter();

    for arg in args.by_ref() {
        match arg.as_str() {
            "--plain" => plain = true,
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
    }))
}

pub const KEY_HELP: &str = "Get a free API key at https://www.deepl.com/pro-api";

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
            Ok(format!("key saved to {} (0600)\n", path.display()))
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
            args(&["--plain", "pl:en", "dzień", "dobry"]).unwrap(),
            Args::Run(Cli {
                pair: "pl:en".into(),
                text: vec!["dzień".into(), "dobry".into()],
                plain: true,
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
    fn help_and_version_short_circuit() {
        assert_eq!(args(&["--help"]).unwrap(), Args::Message(USAGE.to_string()));
        let Ok(Args::Message(v)) = args(&["-V"]) else {
            panic!("expected a message")
        };
        assert!(v.starts_with("translate 0."));
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

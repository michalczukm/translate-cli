use std::io::{IsTerminal, Read, Write};

use translate_cli::cli::{self, Args};
use translate_cli::{config, prompt};

fn main() {
    let cli = match cli::parse_args(std::env::args().skip(1)) {
        Ok(Args::Run(cli)) => cli,
        Ok(Args::Message(text)) => {
            print!("{text}");
            std::process::exit(0)
        }
        Err(msg) => {
            eprint!("{msg}");
            std::process::exit(2)
        }
    };
    let color = !cli.plain && std::io::stdout().is_terminal();

    if cli.pair == "auth" {
        finish(cli::auth_command(&cli.text));
    }

    let key = match config::load() {
        Ok(k) => k,
        Err(msg) => match first_run_key() {
            Some(k) => k,
            None => fail(3, &msg),
        },
    };

    let stdin = if cli::needs_stdin(&cli.pair, &cli.text, std::io::stdin().is_terminal()) {
        let mut buf = String::new();
        let _ = std::io::stdin().read_to_string(&mut buf);
        buf
    } else {
        String::new()
    };

    finish(cli::run(cli, key, color, &stdin));
}

/// Only offer the prompt on a terminal; scripts and pipes keep the exit-3 error.
fn first_run_key() -> Option<String> {
    if !std::io::stderr().is_terminal() {
        return None;
    }

    eprintln!("No DeepL API key found. {}", cli::KEY_HELP);
    let key = prompt::read_secret("DeepL API key: ").ok()?;
    if key.is_empty() {
        return None;
    }

    match config::save(&key) {
        Ok(path) => {
            eprintln!("Saved to {} (0600)", path.display());
            Some(key)
        }
        Err(msg) => fail(1, &msg),
    }
}

fn finish(result: Result<String, (i32, String)>) -> ! {
    match result {
        Ok(out) => {
            // A closed pipe (`| head`) is a normal end, not a panic out of print!.
            match write!(std::io::stdout(), "{out}") {
                Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => fail(1, &e.to_string()),
                _ => std::process::exit(0),
            }
        }
        Err((code, msg)) => fail(code, &msg),
    }
}

fn fail(code: i32, msg: &str) -> ! {
    eprintln!("translate: {msg}");
    std::process::exit(code)
}

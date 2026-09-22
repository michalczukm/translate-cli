use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Reads from /dev/tty, not stdin, so the prompt still works while text is piped in.
pub fn read_secret(label: &str) -> Result<String, String> {
    let tty = File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| format!("no terminal to read the key from: {e}"))?;

    let mut out = tty.try_clone().map_err(|e| e.to_string())?;
    write!(out, "{label}").map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;

    set_echo(&tty, false);

    let mut line = String::new();
    let read = BufReader::new(tty.try_clone().map_err(|e| e.to_string())?).read_line(&mut line);

    set_echo(&tty, true);
    let _ = writeln!(out);

    read.map_err(|e| e.to_string())?;
    Ok(line.trim().to_string())
}

fn set_echo(tty: &File, on: bool) {
    let Ok(stdin) = tty.try_clone() else { return };
    let _ = Command::new("stty")
        .arg(if on { "echo" } else { "-echo" })
        .stdin(Stdio::from(stdin))
        .status();
}

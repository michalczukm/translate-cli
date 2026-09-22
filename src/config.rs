use std::path::PathBuf;

pub fn parse_env_file(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let value = line.strip_prefix("DEEPL_API_KEY=")?;
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

pub fn resolve(env_value: Option<String>, files: &[PathBuf]) -> Option<String> {
    if let Some(v) = env_value
        && !v.trim().is_empty()
    {
        return Some(v.trim().to_string());
    }
    files.iter().find_map(|p| {
        std::fs::read_to_string(p)
            .ok()
            .and_then(|c| parse_env_file(&c))
    })
}

pub fn candidate_files() -> Vec<PathBuf> {
    [config_path(), Some(PathBuf::from(".env"))]
        .into_iter()
        .flatten()
        .collect()
}

pub fn config_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/translate-cli/.env"))
}

/// The key is a secret in a plaintext file, so the directory and file are
/// owner-only, matching ~/.aws/credentials.
pub fn save_to(path: &std::path::Path, key: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};

    let key = key.trim();
    if key.is_empty() {
        return Err("empty API key".to_string());
    }

    if let Some(dir) = path.parent() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?;
    }

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(file, "DEEPL_API_KEY={key}").map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn save(key: &str) -> Result<PathBuf, String> {
    let path = config_path().ok_or("HOME is not set")?;
    save_to(&path, key)?;
    Ok(path)
}

pub fn load() -> Result<String, String> {
    resolve(std::env::var("DEEPL_API_KEY").ok(), &candidate_files()).ok_or_else(|| {
        "no DeepL API key found. Run `translate auth set-key`, or set DEEPL_API_KEY".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_key_from_env_file() {
        let body = "# comment\nOTHER=1\nDEEPL_API_KEY=abc-123:fx\n";
        assert_eq!(parse_env_file(body), Some("abc-123:fx".to_string()));
    }

    #[test]
    fn strips_quotes_and_export_prefix() {
        assert_eq!(
            parse_env_file("export DEEPL_API_KEY=\"k:fx\""),
            Some("k:fx".to_string())
        );
    }

    #[test]
    fn missing_key_is_none() {
        assert_eq!(parse_env_file("NOPE=1"), None);
    }

    #[test]
    fn env_var_wins_over_files() {
        let dir = std::env::temp_dir().join("translate-cli-test-1");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join(".env");
        write!(
            std::fs::File::create(&f).unwrap(),
            "DEEPL_API_KEY=from-file"
        )
        .unwrap();
        assert_eq!(
            resolve(Some("from-env".into()), std::slice::from_ref(&f)),
            Some("from-env".into())
        );
        assert_eq!(resolve(None, &[f]), Some("from-file".into()));
    }

    #[test]
    fn save_to_writes_a_parseable_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("translate-cli-test-save");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join(".env");

        save_to(&path, "abc-123:fx").unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(parse_env_file(&body), Some("abc-123:fx".to_string()));
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn save_to_replaces_an_existing_key() {
        let dir = std::env::temp_dir().join("translate-cli-test-save-2");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join(".env");

        save_to(&path, "old:fx").unwrap();
        save_to(&path, "new:fx").unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(parse_env_file(&body), Some("new:fx".to_string()));
    }

    #[test]
    fn empty_key_is_rejected() {
        let path = std::env::temp_dir().join("translate-cli-test-save-3/.env");
        assert!(save_to(&path, "  ").is_err());
    }

    #[test]
    fn blank_env_var_is_ignored() {
        assert_eq!(resolve(Some("  ".into()), &[]), None);
    }
}

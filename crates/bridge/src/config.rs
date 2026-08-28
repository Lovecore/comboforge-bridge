use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The ONE file this program ever writes. Every filesystem operation in the
/// repository lives in this module -- that is a checkable claim, and the
/// README makes it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub token: String,
    /// Extra allowed origins on top of the compiled-in list. Printed loudly
    /// at startup so a quietly-edited config cannot widen access invisibly.
    #[serde(default)]
    pub extra_origins: Vec<String>,
}

pub fn config_path() -> PathBuf {
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("ComboForge Bridge").join("config.json")
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(".config")
            .join("comboforge-bridge")
            .join("config.json")
    }
}

/// Parse a config file's raw contents. Extracted and pure so the traps that
/// broke the first field test are pinned by tests: Windows editors prepend a
/// UTF-8 BOM when saving, which strict JSON parsers reject at byte one --
/// forgiving it costs nothing and cannot change any parsed value.
pub fn parse_config(raw: &str) -> Result<Config, serde_json::Error> {
    serde_json::from_str::<Config>(raw.trim_start_matches('\u{feff}'))
}

/// Keep the message on screen when the program was double-clicked: a console
/// window that closes with its error unread is indistinguishable from a
/// crash, which is exactly how this path was first reported.
fn pause_before_exit() -> ! {
    eprintln!("  Press Enter to close this window.");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    std::process::exit(1);
}

pub fn load_or_create() -> Config {
    let path = config_path();
    if let Ok(raw) = fs::read_to_string(&path) {
        match parse_config(&raw) {
            Ok(config) => return config,
            Err(error) => {
                // NEVER silently regenerate: that would wipe a hand-edit AND
                // rotate the pairing token, turning one JSON typo into two
                // mysteries. Fail loudly, leave the file exactly as it is.
                eprintln!();
                eprintln!("  Your config file has a JSON error and was NOT loaded:");
                eprintln!("    {}", path.display());
                eprintln!("    {error}");
                eprintln!("  Common causes: a trailing comma after the last entry, or a missing");
                eprintln!("  comma between the token line and extraOrigins.");
                eprintln!("  Fix the JSON (or delete the file to start fresh), then run again.");
                eprintln!();
                pause_before_exit();
            }
        }
    }
    let config = Config {
        token: crate::token::generate(),
        extra_origins: Vec::new(),
    };
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Err(e) = fs::write(&path, serde_json::to_string_pretty(&config).unwrap()) {
        eprintln!(
            "could not write {}: {e} (token will rotate next run)",
            path.display()
        );
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"token":"AAAA-BBBB","extraOrigins":["https://x.example"]}"#;

    #[test]
    fn a_windows_editor_bom_is_forgiven() {
        let with_bom = format!("\u{feff}{GOOD}");
        let config = parse_config(&with_bom).expect("BOM must not break parsing");
        assert_eq!(config.extra_origins, vec!["https://x.example"]);
        assert_eq!(config.token, "AAAA-BBBB");
    }

    #[test]
    fn a_trailing_comma_fails_with_a_named_error_not_silence() {
        let broken = r#"{"token":"AAAA-BBBB","extraOrigins":["https://x.example"],}"#;
        assert!(parse_config(broken).is_err());
    }
}

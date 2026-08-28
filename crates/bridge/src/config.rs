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

pub fn load_or_create() -> Config {
    let path = config_path();
    if let Ok(raw) = fs::read_to_string(&path) {
        match serde_json::from_str::<Config>(&raw) {
            Ok(config) => return config,
            Err(error) => {
                // NEVER silently regenerate: that would wipe a hand-edit AND
                // rotate the pairing token, turning one JSON typo into two
                // mysteries. Fail loudly, leave the file exactly as it is.
                eprintln!();
                eprintln!("  Your config file has a JSON error and was NOT loaded:");
                eprintln!("    {}", path.display());
                eprintln!("    {error}");
                eprintln!("  Fix the JSON (or delete the file to start fresh), then run again.");
                eprintln!();
                std::process::exit(1);
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

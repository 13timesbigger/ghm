use std::path::{Path, PathBuf};

use crate::error::{GhadError, Result};
use crate::models::Config;

/// Default configuration directory name.
const CONFIG_DIR: &str = "ghad";
/// Default configuration file name.
const CONFIG_FILE: &str = "config.json";

/// Returns the default configuration directory: `~/.config/ghad/`.
pub fn default_config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|d| d.join(CONFIG_DIR))
        .ok_or_else(|| GhadError::ConfigInvalid {
            message: "could not determine config directory".into(),
        })
}

/// Returns the default config file path: `~/.config/ghad/config.json`.
pub fn default_config_path() -> Result<PathBuf> {
    Ok(default_config_dir()?.join(CONFIG_FILE))
}

/// Load configuration from the given path, returning defaults if the file does not exist.
pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = std::fs::read_to_string(path).map_err(|e| GhadError::ConfigRead { source: e })?;
    let config: Config =
        serde_json::from_str(&contents).map_err(|e| GhadError::ConfigParse { source: e })?;
    Ok(config)
}

/// Load configuration from the default path.
pub fn load_default_config() -> Result<Config> {
    let path = default_config_path()?;
    load_config(&path)
}

/// Save configuration to the given path, creating parent directories as needed.
pub fn save_config(path: &Path, config: &Config) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GhadError::ConfigWrite { source: e })?;
    }
    let json =
        serde_json::to_string_pretty(config).map_err(|e| GhadError::ConfigParse { source: e })?;
    std::fs::write(path, json).map_err(|e| GhadError::ConfigWrite { source: e })?;
    Ok(())
}

/// Save configuration to the default path.
pub fn save_default_config(config: &Config) -> Result<()> {
    let path = default_config_path()?;
    save_config(&path, config)
}

/// Ensure the config directory and a default config file exist.
pub fn ensure_config(path: &Path) -> Result<Config> {
    if path.exists() {
        load_config(path)
    } else {
        let config = Config::default();
        save_config(path, &config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AgentPaths, AuthMethod};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn temp_config_path(dir: &TempDir) -> PathBuf {
        dir.path().join("config.json")
    }

    #[test]
    fn default_config_dir_exists() {
        // Should not error on macOS / Linux.
        let dir = default_config_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("ghad"));
    }

    #[test]
    fn default_config_path_has_json() {
        let path = default_config_path().unwrap();
        assert!(path.ends_with("config.json"));
    }

    #[test]
    fn load_config_missing_file_returns_default() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        let config = load_config(&path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        let config = Config {
            github_token: Some("ghp_abc123".into()),
            auth_method: AuthMethod::DeviceFlow,
            default_poll_interval_secs: 45,
            default_working_dir: Some(PathBuf::from("/projects")),
            agent_paths: AgentPaths {
                codex: Some(PathBuf::from("/usr/local/bin/codex")),
                ..Default::default()
            },
        };
        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn save_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sub").join("deep").join("config.json");
        let config = Config::default();
        save_config(&path, &config).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn load_config_invalid_json() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        std::fs::write(&path, "NOT JSON").unwrap();
        let result = load_config(&path);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("parse config"));
    }

    #[test]
    fn ensure_config_creates_new() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        let config = ensure_config(&path).unwrap();
        assert_eq!(config, Config::default());
        assert!(path.exists());
    }

    #[test]
    fn ensure_config_loads_existing() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        let config = Config {
            github_token: Some("token".into()),
            ..Default::default()
        };
        save_config(&path, &config).unwrap();
        let loaded = ensure_config(&path).unwrap();
        assert_eq!(loaded.github_token, Some("token".into()));
    }

    #[test]
    fn save_config_overwrites() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);

        let c1 = Config {
            github_token: Some("first".into()),
            ..Default::default()
        };
        save_config(&path, &c1).unwrap();

        let c2 = Config {
            github_token: Some("second".into()),
            ..Default::default()
        };
        save_config(&path, &c2).unwrap();

        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.github_token, Some("second".into()));
    }

    #[test]
    fn config_partial_json_uses_defaults() {
        let tmp = TempDir::new().unwrap();
        let path = temp_config_path(&tmp);
        std::fs::write(&path, r#"{"github_token": "tok"}"#).unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.github_token, Some("tok".into()));
        assert_eq!(config.default_poll_interval_secs, 30);
        assert_eq!(config.auth_method, AuthMethod::PersonalAccessToken);
    }
}

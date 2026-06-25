use std::path::PathBuf;
use thiserror::Error;

/// Top-level error type for the ghm-core library.
#[derive(Debug, Error)]
pub enum GhmError {
    // ── Config errors ──────────────────────────────────────────────
    #[error("config file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[error("failed to read config: {source}")]
    ConfigRead {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write config: {source}")]
    ConfigWrite {
        #[source]
        source: std::io::Error,
    },

    #[error("invalid config: {message}")]
    ConfigInvalid { message: String },

    #[error("failed to parse config JSON: {source}")]
    ConfigParse {
        #[source]
        source: serde_json::Error,
    },

    // ── Store errors ───────────────────────────────────────────────
    #[error("store file not found: {path}")]
    StoreNotFound { path: PathBuf },

    #[error("failed to read store: {source}")]
    StoreRead {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write store: {source}")]
    StoreWrite {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse store JSON: {source}")]
    StoreParse {
        #[source]
        source: serde_json::Error,
    },

    #[error("atomic rename failed from {from} to {to}: {source}")]
    AtomicRename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },

    // ── GitHub errors ──────────────────────────────────────────────
    #[error("GitHub authentication failed: {message}")]
    AuthFailed { message: String },

    #[error("GitHub API error: {message}")]
    GitHubApi { message: String },

    #[error("GitHub token not configured")]
    TokenMissing,

    #[error("device flow error: {message}")]
    DeviceFlow { message: String },

    #[error("GraphQL error: {message}")]
    GraphQL { message: String },

    // ── Daemon errors ──────────────────────────────────────────────
    #[error("daemon already running (pid={pid})")]
    DaemonAlreadyRunning { pid: u32 },

    #[error("daemon not running")]
    DaemonNotRunning,

    #[error("agent dispatch failed for {agent}: {message}")]
    AgentDispatch { agent: String, message: String },

    #[error("launchd plist error: {message}")]
    LaunchdError { message: String },

    #[error("poll error: {message}")]
    PollError { message: String },

    // ── Generic wrappers ───────────────────────────────────────────
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("{0}")]
    Other(String),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, GhmError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn config_not_found_display() {
        let err = GhmError::ConfigNotFound {
            path: PathBuf::from("/tmp/missing.json"),
        };
        assert!(err.to_string().contains("/tmp/missing.json"));
    }

    #[test]
    fn config_invalid_display() {
        let err = GhmError::ConfigInvalid {
            message: "bad token".into(),
        };
        assert!(err.to_string().contains("bad token"));
    }

    #[test]
    fn store_not_found_display() {
        let err = GhmError::StoreNotFound {
            path: PathBuf::from("/tmp/store.json"),
        };
        assert!(err.to_string().contains("/tmp/store.json"));
    }

    #[test]
    fn auth_failed_display() {
        let err = GhmError::AuthFailed {
            message: "invalid token".into(),
        };
        assert!(err.to_string().contains("invalid token"));
    }

    #[test]
    fn github_api_display() {
        let err = GhmError::GitHubApi {
            message: "rate limited".into(),
        };
        assert!(err.to_string().contains("rate limited"));
    }

    #[test]
    fn token_missing_display() {
        let err = GhmError::TokenMissing;
        assert!(err.to_string().contains("not configured"));
    }

    #[test]
    fn device_flow_display() {
        let err = GhmError::DeviceFlow {
            message: "timeout".into(),
        };
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn graphql_display() {
        let err = GhmError::GraphQL {
            message: "query failed".into(),
        };
        assert!(err.to_string().contains("query failed"));
    }

    #[test]
    fn daemon_already_running_display() {
        let err = GhmError::DaemonAlreadyRunning { pid: 1234 };
        assert!(err.to_string().contains("1234"));
    }

    #[test]
    fn daemon_not_running_display() {
        let err = GhmError::DaemonNotRunning;
        assert!(err.to_string().contains("not running"));
    }

    #[test]
    fn agent_dispatch_display() {
        let err = GhmError::AgentDispatch {
            agent: "codex".into(),
            message: "not found".into(),
        };
        let s = err.to_string();
        assert!(s.contains("codex"));
        assert!(s.contains("not found"));
    }

    #[test]
    fn launchd_display() {
        let err = GhmError::LaunchdError {
            message: "plist bad".into(),
        };
        assert!(err.to_string().contains("plist bad"));
    }

    #[test]
    fn poll_error_display() {
        let err = GhmError::PollError {
            message: "network".into(),
        };
        assert!(err.to_string().contains("network"));
    }

    #[test]
    fn io_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: GhmError = io_err.into();
        assert!(err.to_string().contains("gone"));
    }

    #[test]
    fn json_from() {
        let json_err = serde_json::from_str::<String>("not json").unwrap_err();
        let err: GhmError = json_err.into();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn other_display() {
        let err = GhmError::Other("custom error".into());
        assert!(err.to_string().contains("custom error"));
    }

    #[test]
    fn config_read_display() {
        let err = GhmError::ConfigRead {
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(err.to_string().contains("read config"));
    }

    #[test]
    fn config_write_display() {
        let err = GhmError::ConfigWrite {
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(err.to_string().contains("write config"));
    }

    #[test]
    fn config_parse_display() {
        let json_err = serde_json::from_str::<String>("bad").unwrap_err();
        let err = GhmError::ConfigParse { source: json_err };
        assert!(err.to_string().contains("parse config"));
    }

    #[test]
    fn store_read_display() {
        let err = GhmError::StoreRead {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no file"),
        };
        assert!(err.to_string().contains("read store"));
    }

    #[test]
    fn store_write_display() {
        let err = GhmError::StoreWrite {
            source: std::io::Error::new(std::io::ErrorKind::Other, "disk full"),
        };
        assert!(err.to_string().contains("write store"));
    }

    #[test]
    fn store_parse_display() {
        let json_err = serde_json::from_str::<String>("x").unwrap_err();
        let err = GhmError::StoreParse { source: json_err };
        assert!(err.to_string().contains("parse store"));
    }

    #[test]
    fn atomic_rename_display() {
        let err = GhmError::AtomicRename {
            from: PathBuf::from("/tmp/a"),
            to: PathBuf::from("/tmp/b"),
            source: std::io::Error::new(std::io::ErrorKind::Other, "fail"),
        };
        let s = err.to_string();
        assert!(s.contains("/tmp/a"));
        assert!(s.contains("/tmp/b"));
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(GhmError::TokenMissing);
        assert!(err.is_err());
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        // GhmError should be Send + Sync for async usage
        assert_send_sync::<GhmError>();
    }

    #[test]
    fn debug_impl() {
        let err = GhmError::TokenMissing;
        let debug = format!("{:?}", err);
        assert!(debug.contains("TokenMissing"));
    }
}

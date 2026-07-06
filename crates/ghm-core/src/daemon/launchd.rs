use std::path::{Path, PathBuf};

use crate::error::{GhmError, Result};

/// Label used in the plist.
const PLIST_LABEL: &str = "com.ghad.daemon";

/// Manages macOS launchd plist generation for the GHM daemon.
#[derive(Debug, Clone)]
pub struct LaunchdManager {
    /// Path to the ghad binary.
    binary_path: PathBuf,
    /// Path to the config directory.
    config_dir: PathBuf,
}

impl LaunchdManager {
    pub fn new(binary_path: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            binary_path,
            config_dir,
        }
    }

    /// Get the label used in the plist file.
    pub fn label(&self) -> &str {
        PLIST_LABEL
    }

    /// Default plist install path: `~/Library/LaunchAgents/com.ghad.daemon.plist`.
    pub fn default_plist_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| GhmError::LaunchdError {
            message: "could not determine home directory".into(),
        })?;
        Ok(home
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{PLIST_LABEL}.plist")))
    }

    /// Generate the plist XML content.
    pub fn generate_plist(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
        <string>daemon</string>
        <string>run</string>
        <string>--config-dir</string>
        <string>{config_dir}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>{config_dir}/daemon.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{config_dir}/daemon.stderr.log</string>
</dict>
</plist>
"#,
            label = PLIST_LABEL,
            binary = self.binary_path.display(),
            config_dir = self.config_dir.display(),
        )
    }

    /// Write the plist to the given path.
    pub fn install_plist(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| GhmError::LaunchdError {
                message: format!("failed to create plist directory: {e}"),
            })?;
        }
        let content = self.generate_plist();
        std::fs::write(path, content).map_err(|e| GhmError::LaunchdError {
            message: format!("failed to write plist: {e}"),
        })?;
        Ok(())
    }

    /// Remove the plist from the given path.
    pub fn uninstall_plist(&self, path: &Path) -> Result<()> {
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| GhmError::LaunchdError {
                message: format!("failed to remove plist: {e}"),
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_manager() -> LaunchdManager {
        LaunchdManager::new(
            PathBuf::from("/usr/local/bin/ghad"),
            PathBuf::from("/Users/test/.config/ghm"),
        )
    }

    #[test]
    fn label_is_correct() {
        let mgr = make_manager();
        assert_eq!(mgr.label(), "com.ghad.daemon");
    }

    #[test]
    fn default_plist_path_has_correct_name() {
        let path = LaunchdManager::default_plist_path();
        // Should succeed on macOS
        if let Ok(p) = path {
            assert!(p.ends_with("com.ghad.daemon.plist"));
            assert!(p.to_string_lossy().contains("LaunchAgents"));
        }
    }

    #[test]
    fn generate_plist_contains_required_keys() {
        let mgr = make_manager();
        let plist = mgr.generate_plist();

        assert!(plist.contains("<?xml version"));
        assert!(plist.contains("com.ghad.daemon"));
        assert!(plist.contains("/usr/local/bin/ghad"));
        assert!(plist.contains("daemon"));
        assert!(plist.contains("run"));
        assert!(plist.contains("--config-dir"));
        assert!(plist.contains("/Users/test/.config/ghm"));
        assert!(plist.contains("RunAtLoad"));
        assert!(plist.contains("KeepAlive"));
        assert!(plist.contains("StandardOutPath"));
        assert!(plist.contains("StandardErrorPath"));
        assert!(plist.contains("daemon.stdout.log"));
        assert!(plist.contains("daemon.stderr.log"));
    }

    #[test]
    fn generate_plist_valid_xml_structure() {
        let mgr = make_manager();
        let plist = mgr.generate_plist();
        // Basic XML structure checks
        assert!(plist.starts_with("<?xml"));
        assert!(plist.contains("<plist version=\"1.0\">"));
        assert!(plist.contains("</plist>"));
        assert!(plist.contains("<dict>"));
        assert!(plist.contains("</dict>"));
    }

    #[test]
    fn install_plist_creates_file() {
        let tmp = TempDir::new().unwrap();
        let mgr = make_manager();
        let path = tmp.path().join("com.ghad.daemon.plist");
        mgr.install_plist(&path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("com.ghad.daemon"));
    }

    #[test]
    fn install_plist_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let mgr = make_manager();
        let path = tmp
            .path()
            .join("Library")
            .join("LaunchAgents")
            .join("com.ghad.daemon.plist");
        mgr.install_plist(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn uninstall_plist_removes_file() {
        let tmp = TempDir::new().unwrap();
        let mgr = make_manager();
        let path = tmp.path().join("com.ghad.daemon.plist");
        mgr.install_plist(&path).unwrap();
        assert!(path.exists());
        mgr.uninstall_plist(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn uninstall_plist_nonexistent_ok() {
        let tmp = TempDir::new().unwrap();
        let mgr = make_manager();
        let path = tmp.path().join("nonexistent.plist");
        // Should not error
        mgr.uninstall_plist(&path).unwrap();
    }

    #[test]
    fn manager_is_clone_debug() {
        let mgr = make_manager();
        let mgr2 = mgr.clone();
        let _ = format!("{:?}", mgr2);
    }

    #[test]
    fn plist_with_spaces_in_paths() {
        let mgr = LaunchdManager::new(
            PathBuf::from("/usr/local/bin/my ghad"),
            PathBuf::from("/Users/my user/.config/ghm"),
        );
        let plist = mgr.generate_plist();
        assert!(plist.contains("/usr/local/bin/my ghad"));
        assert!(plist.contains("/Users/my user/.config/ghm"));
    }
}

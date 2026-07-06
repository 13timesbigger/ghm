pub mod dispatcher;
pub mod launchd;
pub mod poller;
pub mod processor;

use std::path::PathBuf;
use std::time::Duration;
use anyhow::{Context, Result};

use crate::config;
use crate::store::StateStore;
use launchd::LaunchdManager;

pub struct DaemonManager {
    config_dir: PathBuf,
    state_store: StateStore,
    launchd: LaunchdManager,
}

impl DaemonManager {
    pub fn new() -> Result<Self> {
        let config_dir = config::default_config_dir()?;
        let state_store = StateStore::new(&config_dir);
        
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ghad"));
        let launchd = LaunchdManager::new(exe_path, config_dir.clone());
        
        Ok(Self {
            config_dir,
            state_store,
            launchd,
        })
    }

    pub fn is_running(&self) -> bool {
        self.pid().is_some()
    }

    pub fn pid(&self) -> Option<u32> {
        self.state_store.load().ok().and_then(|s| s.daemon_pid)
    }

    pub async fn start(&self) -> Result<()> {
        let plist_path = LaunchdManager::default_plist_path()?;
        self.launchd.install_plist(&plist_path)?;

        // Use launchctl to load/start it
        let output = std::process::Command::new("launchctl")
            .arg("load")
            .arg("-w")
            .arg(&plist_path)
            .output()
            .context("Failed to run launchctl load")?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let details = [stdout, stderr]
                .into_iter()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if details.is_empty() {
                anyhow::bail!("launchctl load failed with status: {}", output.status);
            }
            anyhow::bail!(
                "launchctl load failed with status: {}\n{}",
                output.status,
                details
            );
        }

        for _ in 0..20 {
            if self.is_running() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!(
            "launchd accepted the daemon job, but the daemon did not report running. Check the daemon stderr log for details."
        );
    }

    pub async fn stop(&self) -> Result<()> {
        let plist_path = LaunchdManager::default_plist_path()?;
        if plist_path.exists() {
            let _ = std::process::Command::new("launchctl")
                .arg("unload")
                .arg("-w")
                .arg(&plist_path)
                .status();
        }
        self.state_store.set_stopped()?;
        Ok(())
    }

    pub fn uptime(&self) -> Option<Duration> {
        // We could track start time in state, but this is a placeholder.
        None
    }

    pub async fn install(&self) -> Result<()> {
        let plist_path = LaunchdManager::default_plist_path()?;
        Ok(self.launchd.install_plist(&plist_path)?)
    }

    pub async fn uninstall(&self) -> Result<()> {
        let _ = self.stop().await;
        let plist_path = LaunchdManager::default_plist_path()?;
        Ok(self.launchd.uninstall_plist(&plist_path)?)
    }
}

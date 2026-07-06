use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use tracing;

use crate::config;
use crate::daemon::processor::EventProcessor;
use crate::error::{GhadError, Result};
use crate::store::{ObservedStore, StateStore};

/// Default polling interval in seconds.
pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;

/// The polling engine that periodically checks observed repos for new events.
pub struct Poller {
    config_dir: std::path::PathBuf,
    interval: Duration,
    shutdown_rx: watch::Receiver<bool>,
}

impl Poller {
    /// Create a new poller.
    ///
    /// - `config_dir`: directory containing config/store JSON files
    /// - `interval`: polling interval (defaults to 30s if `None`)
    /// - `shutdown_rx`: watch channel receiver to signal shutdown
    pub fn new(
        config_dir: std::path::PathBuf,
        interval: Option<Duration>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Self {
        Self {
            config_dir,
            interval: interval.unwrap_or(Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS)),
            shutdown_rx,
        }
    }

    /// Get the polling interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Run the polling loop until shutdown is signalled.
    pub async fn run(&mut self, processor: Arc<EventProcessor>) -> Result<()> {
        let observed_store = ObservedStore::new(&self.config_dir);
        let state_store = StateStore::new(&self.config_dir);

        // Record that we're running
        let pid = std::process::id();
        state_store.set_running(pid)?;

        tracing::info!(
            "Poller started (pid={}, interval={:?})",
            pid,
            self.interval
        );

        loop {
            // Check for shutdown
            if *self.shutdown_rx.borrow() {
                tracing::info!("Shutdown signal received, stopping poller");
                break;
            }

            // Do one poll cycle
            if let Err(e) = self.poll_once(&observed_store, &state_store, &processor).await {
                tracing::error!("Poll cycle error: {e}");
            }

            // Wait for the interval or a shutdown signal
            tokio::select! {
                _ = tokio::time::sleep(self.interval) => {},
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        tracing::info!("Shutdown signal during sleep, stopping poller");
                        break;
                    }
                }
            }
        }

        state_store.set_stopped()?;
        tracing::info!("Poller stopped");
        Ok(())
    }

    /// Execute a single poll cycle across all observed repos.
    async fn poll_once(
        &self,
        observed_store: &ObservedStore,
        state_store: &StateStore,
        processor: &EventProcessor,
    ) -> Result<()> {
        let observed = observed_store.load()?;
        if observed.repositories.is_empty() {
            tracing::debug!("No observed repositories, skipping poll");
            return Ok(());
        }

        for repo in &observed.repositories {
            tracing::debug!("Polling {}", repo.full_name);
            if let Err(e) = processor.process_repo(repo, state_store).await {
                tracing::warn!("Error processing {}: {e}", repo.full_name);
            }
        }

        // Update global last_poll
        let mut state = state_store.load()?;
        state.last_poll = Some(chrono::Utc::now());
        state_store.save(&state)?;

        Ok(())
    }
}

/// Create a shutdown channel pair.
pub fn shutdown_channel() -> (watch::Sender<bool>, watch::Receiver<bool>) {
    watch::channel(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_poll_interval() {
        assert_eq!(DEFAULT_POLL_INTERVAL_SECS, 30);
    }

    #[test]
    fn poller_custom_interval() {
        let (_tx, rx) = shutdown_channel();
        let poller = Poller::new(
            PathBuf::from("/tmp/ghad"),
            Some(Duration::from_secs(60)),
            rx,
        );
        assert_eq!(poller.interval(), Duration::from_secs(60));
    }

    #[test]
    fn poller_default_interval() {
        let (_tx, rx) = shutdown_channel();
        let poller = Poller::new(PathBuf::from("/tmp/ghad"), None, rx);
        assert_eq!(poller.interval(), Duration::from_secs(30));
    }

    #[test]
    fn shutdown_channel_initial_false() {
        let (_tx, rx) = shutdown_channel();
        assert!(!*rx.borrow());
    }

    #[test]
    fn shutdown_channel_signal() {
        let (tx, rx) = shutdown_channel();
        tx.send(true).unwrap();
        assert!(*rx.borrow());
    }

    #[tokio::test]
    async fn poller_immediate_shutdown() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let (tx, rx) = shutdown_channel();
        // Signal shutdown immediately
        tx.send(true).unwrap();

        let processor = Arc::new(EventProcessor::new(tmp.path().to_path_buf()));
        let mut poller = Poller::new(
            tmp.path().to_path_buf(),
            Some(Duration::from_millis(100)),
            rx,
        );
        let result = poller.run(processor).await;
        assert!(result.is_ok());

        // Should have set stopped
        let state_store = StateStore::new(tmp.path());
        let state = state_store.load().unwrap();
        assert_eq!(state.daemon_status, Some("stopped".into()));
    }

    #[tokio::test]
    async fn poller_polls_empty_repos() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let (tx, rx) = shutdown_channel();

        let processor = Arc::new(EventProcessor::new(tmp.path().to_path_buf()));
        let observed_store = ObservedStore::new(tmp.path());
        let state_store = StateStore::new(tmp.path());

        // Save empty observed repos
        observed_store.save(&crate::models::ObservedRepos::default()).unwrap();

        // Run one cycle then shutdown
        let mut poller = Poller::new(
            tmp.path().to_path_buf(),
            Some(Duration::from_millis(50)),
            rx,
        );

        // Spawn poller and shut down after a short delay
        let handle = tokio::spawn(async move {
            poller.run(processor).await
        });

        tokio::time::sleep(Duration::from_millis(150)).await;
        tx.send(true).unwrap();

        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

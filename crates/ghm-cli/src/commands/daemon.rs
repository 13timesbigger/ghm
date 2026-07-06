use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::daemon::DaemonManager;

use crate::cli::DaemonCommands;
use crate::output;

/// Handle daemon subcommands.
pub async fn handle_daemon(cmd: &DaemonCommands) -> Result<()> {
    match cmd {
        DaemonCommands::Start => handle_start().await,
        DaemonCommands::Stop => handle_stop().await,
        DaemonCommands::Status => handle_status().await,
        DaemonCommands::Install => handle_install().await,
        DaemonCommands::Uninstall => handle_uninstall().await,
        DaemonCommands::Run { config_dir: _ } => {
            let manager = DaemonManager::new()?;
            manager.start().await
        },
    }
}

/// Start the daemon process.
async fn handle_start() -> Result<()> {
    let _config = load_default_config().context(
        "Failed to load configuration. Run 'ghad auth configure' first.",
    )?;

    let sp = output::spinner("Starting daemon...");

    let manager = DaemonManager::new()?;

    if manager.is_running() {
        sp.finish_and_clear();
        output::print_warning("Daemon is already running.");
        if let Some(pid) = manager.pid() {
            output::print_info(&format!("PID: {}", pid));
        }
        return Ok(());
    }

    manager.start().await.context("Failed to start daemon")?;

    sp.finish_and_clear();
    output::print_success("Daemon started successfully.");
    if let Some(pid) = manager.pid() {
        output::print_info(&format!("PID: {}", pid));
    }

    Ok(())
}

/// Stop the daemon process.
async fn handle_stop() -> Result<()> {
    let sp = output::spinner("Stopping daemon...");

    let manager = DaemonManager::new()?;

    if !manager.is_running() {
        sp.finish_and_clear();
        output::print_warning("Daemon is not currently running.");
        return Ok(());
    }

    manager.stop().await.context("Failed to stop daemon")?;

    sp.finish_and_clear();
    output::print_success("Daemon stopped.");

    Ok(())
}

/// Check and display daemon status.
async fn handle_status() -> Result<()> {
    let manager = DaemonManager::new()?;

    let running = manager.is_running();
    let pid = manager.pid();
    let uptime = manager.uptime().map(|d| format_duration(d));

    output::print_daemon_status(running, pid, uptime.as_deref());

    Ok(())
}

/// Install the daemon as a system service.
async fn handle_install() -> Result<()> {
    let sp = output::spinner("Installing daemon service...");

    let manager = DaemonManager::new()?;
    manager
        .install()
        .await
        .context("Failed to install daemon service")?;

    sp.finish_and_clear();
    output::print_success("Daemon service installed.");
    output::print_info("Run 'ghad daemon start' to start the daemon.");

    Ok(())
}

/// Uninstall the daemon system service.
async fn handle_uninstall() -> Result<()> {
    let sp = output::spinner("Uninstalling daemon service...");

    let manager = DaemonManager::new()?;
    manager
        .uninstall()
        .await
        .context("Failed to uninstall daemon service")?;

    sp.finish_and_clear();
    output::print_success("Daemon service uninstalled.");

    Ok(())
}

/// Format a duration into a human-readable string.
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_seconds() {
        let d = std::time::Duration::from_secs(45);
        assert_eq!(format_duration(d), "45s");
    }

    #[test]
    fn test_format_duration_minutes() {
        let d = std::time::Duration::from_secs(125);
        assert_eq!(format_duration(d), "2m 5s");
    }

    #[test]
    fn test_format_duration_hours() {
        let d = std::time::Duration::from_secs(3661);
        assert_eq!(format_duration(d), "1h 1m 1s");
    }

    #[test]
    fn test_format_duration_zero() {
        let d = std::time::Duration::from_secs(0);
        assert_eq!(format_duration(d), "0s");
    }

    #[test]
    fn test_format_duration_exact_hour() {
        let d = std::time::Duration::from_secs(7200);
        assert_eq!(format_duration(d), "2h 0m 0s");
    }

    #[test]
    fn test_format_duration_exact_minute() {
        let d = std::time::Duration::from_secs(60);
        assert_eq!(format_duration(d), "1m 0s");
    }
}

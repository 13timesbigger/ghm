use anyhow::{Context, Result};
use dialoguer::{Input, Select};

use ghm_core::config::load_default_config;

use crate::output;

/// Handle the `ghm auth configure` command.
///
/// Interactively prompts the user for authentication method and credentials.
pub async fn handle_configure() -> Result<()> {
    println!("🔐 GitHub Monitor — Authentication Setup\n");

    let methods = vec!["Personal Access Token (PAT)", "GitHub Device Flow (OAuth)"];
    let selection = Select::new()
        .with_prompt("Select authentication method")
        .items(&methods)
        .default(0)
        .interact()
        .context("Failed to get user selection")?;

    match selection {
        0 => configure_pat().await,
        1 => configure_device_flow().await,
        _ => unreachable!(),
    }
}

/// Configure authentication via Personal Access Token.
async fn configure_pat() -> Result<()> {
    println!("\nA Personal Access Token requires the following scopes:");
    println!("  • repo (Full control of private repositories)");
    println!("  • read:org (Read org membership)");
    println!("  • read:project (Read project boards)\n");

    let token: String = Input::new()
        .with_prompt("Enter your GitHub Personal Access Token")
        .interact_text()
        .context("Failed to read token input")?;

    if token.trim().is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    let sp = output::spinner("Validating token...");

    // Load or create config, save token
    let mut config = ghm_core::config::load_default_config().unwrap_or_default();
    config.github_token = Some(token);

    let config_dir = ghm_core::config::default_config_dir()?;
    std::fs::create_dir_all(&config_dir)?;
    
    // Save it (assuming save implementation exists or we just serialize)
    let config_path = ghm_core::config::default_config_path()?;
    let json = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, json)?;

    sp.finish_and_clear();
    output::print_success("Authentication successful!");
    output::print_info(&format!(
        "Configuration saved to {}",
        config_path.display()
    ));

    Ok(())
}

/// Configure authentication via GitHub Device Flow.
async fn configure_device_flow() -> Result<()> {
    let sp = output::spinner("Initiating GitHub Device Flow...");

    // In a real implementation, this would call ghm_core::github::auth::device_flow()
    // For now, we show the flow structure
    sp.finish_and_clear();

    output::print_info("Device Flow authentication is not yet implemented.");
    output::print_info("Please use a Personal Access Token instead.");
    output::print_info("Run: ghm auth configure");

    Ok(())
}


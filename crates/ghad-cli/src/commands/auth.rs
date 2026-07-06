use anyhow::{Context, Result};
use dialoguer::{Input, Password, Select};
use tokio::time::{sleep, Duration, Instant};

use ghad_core::github::auth::{
    poll_device_token, request_device_code, validate_pat_format, verify_pat,
};
use ghad_core::models::AuthMethod;

use crate::output;

const DEVICE_CLIENT_ID_ENV: &str = "GHAD_GITHUB_CLIENT_ID";
const DEVICE_FLOW_SCOPES: &str = "repo read:org read:project";

/// Handle the `ghad auth configure` command.
///
/// Interactively prompts the user for authentication method and credentials.
pub async fn handle_configure() -> Result<()> {
    println!("🔐 GHAAD — Authentication Setup\n");

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

    let token: String = Password::new()
        .with_prompt("Enter your GitHub Personal Access Token")
        .interact()
        .context("Failed to read token input")?;

    if token.trim().is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    validate_pat_format(token.trim()).context("Invalid GitHub Personal Access Token")?;

    let sp = output::spinner("Validating token...");
    let login = match verify_pat(token.trim()).await {
        Ok(login) => login,
        Err(err) => {
            sp.finish_and_clear();
            return Err(err).context("GitHub rejected the provided token");
        }
    };

    save_token(token.trim(), AuthMethod::PersonalAccessToken)?;

    sp.finish_and_clear();
    output::print_success(&format!("Authentication successful for @{login}!"));

    let config_path = ghad_core::config::default_config_path()?;
    output::print_info(&format!("Configuration saved to {}", config_path.display()));

    Ok(())
}

/// Configure authentication via GitHub Device Flow.
async fn configure_device_flow() -> Result<()> {
    let client_id = resolve_device_client_id()?;

    let sp = output::spinner("Initiating GitHub Device Flow...");
    let device = match request_device_code(&client_id, DEVICE_FLOW_SCOPES).await {
        Ok(device) => device,
        Err(err) => {
            sp.finish_and_clear();
            return Err(err).context(
                "Failed to start GitHub Device Flow. Make sure you entered the OAuth App client ID, not the client secret, and that Device Flow is enabled in the app settings",
            );
        }
    };
    sp.finish_and_clear();

    output::print_info("Open this URL in your browser:");
    println!("{}", device.verification_uri);
    output::print_info("Enter this code:");
    println!("{}", device.user_code);
    output::print_info("Waiting for GitHub authorization...");

    let deadline = Instant::now() + Duration::from_secs(device.expires_in);
    let mut interval = Duration::from_secs(device.interval.max(1));
    let sp = output::spinner("Waiting for authorization...");

    while Instant::now() < deadline {
        sleep(interval).await;

        let token_response = match poll_device_token(&client_id, &device.device_code).await {
            Ok(response) => response,
            Err(err) => {
                sp.finish_and_clear();
                return Err(err).context("Failed while polling GitHub Device Flow");
            }
        };

        if let Some(token) = token_response.access_token {
            save_token(&token, AuthMethod::DeviceFlow)?;
            sp.finish_and_clear();

            let config_path = ghad_core::config::default_config_path()?;
            output::print_success("Authentication successful!");
            output::print_info(&format!("Configuration saved to {}", config_path.display()));
            return Ok(());
        }

        match token_response.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += Duration::from_secs(5),
            Some("expired_token") => {
                sp.finish_and_clear();
                anyhow::bail!("GitHub Device Flow code expired. Run 'ghad auth configure' again.");
            }
            Some("access_denied") => {
                sp.finish_and_clear();
                anyhow::bail!("GitHub Device Flow authorization was denied.");
            }
            Some(error) => {
                sp.finish_and_clear();
                let description = token_response
                    .error_description
                    .unwrap_or_else(|| "No error description provided".to_string());
                anyhow::bail!("GitHub Device Flow failed: {error}: {description}");
            }
            None => {}
        }
    }

    sp.finish_and_clear();
    anyhow::bail!("GitHub Device Flow timed out. Run 'ghad auth configure' again.");
}

fn resolve_device_client_id() -> Result<String> {
    if let Some(client_id) = std::env::var(DEVICE_CLIENT_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(client_id);
    }

    println!("\nGitHub Device Flow requires a GitHub OAuth App client ID.");
    println!("Use the OAuth App client ID, not the client secret.");
    println!("Device Flow must also be enabled in the app settings.");
    println!("You can set the client ID with {DEVICE_CLIENT_ID_ENV}.\n");

    let client_id: String = Input::new()
        .with_prompt("Enter your GitHub OAuth App client ID")
        .interact_text()
        .context("Failed to read GitHub OAuth app client ID")?;

    let client_id = client_id.trim().to_string();
    if client_id.is_empty() {
        anyhow::bail!("GitHub OAuth app client ID cannot be empty");
    }

    Ok(client_id)
}

fn save_token(token: &str, auth_method: AuthMethod) -> Result<()> {
    let mut config = ghad_core::config::load_default_config().unwrap_or_default();
    config.github_token = Some(token.to_string());
    config.auth_method = auth_method;

    ghad_core::config::save_default_config(&config).context("Failed to save GitHub credentials")?;
    Ok(())
}

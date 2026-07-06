use anyhow::{Context, Result};
use dialoguer::{Password, Select};
use std::collections::BTreeSet;
use std::process::Command;
use tokio::time::{sleep, Duration, Instant};

use ghad_core::github::auth::{
    poll_device_token, request_device_code, validate_pat_format, verify_pat,
};
use ghad_core::models::AuthMethod;

use crate::output;

const DEVICE_CLIENT_ID_ENV: &str = "GHAD_GITHUB_CLIENT_ID";
const DEFAULT_DEVICE_CLIENT_ID: &str = "Iv23liLPhwgYwYeBHjoX";
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
    let client_id = resolve_device_client_id();

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

    match open_url_in_default_browser(&device.verification_uri) {
        Ok(()) => output::print_info("Opened GitHub Device Flow in your default browser."),
        Err(err) => output::print_warning(&format!("Could not open browser automatically: {err}")),
    }
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
            validate_granted_device_scopes(token_response.scope.as_deref())?;
            save_token(&token, AuthMethod::DeviceFlow)?;
            sp.finish_and_clear();

            let config_path = ghad_core::config::default_config_path()?;
            output::print_success("Authentication successful!");
            output::print_info(&format!(
                "Granted GitHub scopes: {}",
                token_response.scope.as_deref().unwrap_or("(none)")
            ));
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

fn resolve_device_client_id() -> String {
    if let Some(client_id) = std::env::var(DEVICE_CLIENT_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return client_id;
    }

    DEFAULT_DEVICE_CLIENT_ID.to_string()
}

fn open_url_in_default_browser(url: &str) -> Result<()> {
    let status = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).status()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).status()
    } else {
        Command::new("xdg-open").arg(url).status()
    }
    .context("failed to launch browser command")?;

    if !status.success() {
        anyhow::bail!("browser command exited with status {status}");
    }

    Ok(())
}

fn validate_granted_device_scopes(granted_scope: Option<&str>) -> Result<()> {
    let granted = parse_scopes(granted_scope.unwrap_or_default());
    let required = parse_scopes(DEVICE_FLOW_SCOPES);
    let missing: Vec<_> = required.difference(&granted).cloned().collect();

    if missing.is_empty() {
        return Ok(());
    }

    let granted_display = if granted.is_empty() {
        "(none)".to_string()
    } else {
        granted.into_iter().collect::<Vec<_>>().join(", ")
    };

    anyhow::bail!(
        "GitHub did not grant the required OAuth scopes. Missing: {}. Granted: {}. Re-run 'ghad auth configure' and approve all requested scopes; if this is an organization repository, the organization may also need to approve the OAuth app.",
        missing.join(", "),
        granted_display
    );
}

fn parse_scopes(scopes: &str) -> BTreeSet<String> {
    scopes
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn save_token(token: &str, auth_method: AuthMethod) -> Result<()> {
    let mut config = ghad_core::config::load_default_config().unwrap_or_default();
    config.github_token = Some(token.to_string());
    config.auth_method = auth_method;

    ghad_core::config::save_default_config(&config).context("Failed to save GitHub credentials")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scopes_accepts_space_and_comma_separators() {
        let scopes = parse_scopes("repo, read:org read:project");
        assert!(scopes.contains("repo"));
        assert!(scopes.contains("read:org"));
        assert!(scopes.contains("read:project"));
    }

    #[test]
    fn validate_granted_device_scopes_accepts_required_scopes() {
        validate_granted_device_scopes(Some("repo read:org read:project")).unwrap();
    }

    #[test]
    fn validate_granted_device_scopes_rejects_empty_scopes() {
        let err = validate_granted_device_scopes(Some("")).unwrap_err();
        assert!(err.to_string().contains("Missing:"));
        assert!(err.to_string().contains("Granted: (none)"));
    }
}

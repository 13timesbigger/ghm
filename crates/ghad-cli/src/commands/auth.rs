use anyhow::{Context, Result};
use dialoguer::{Input, Select};
use std::path::Path;
use std::process::Command;

use ghad_core::github::client::list_github_app_installations;
use ghad_core::models::{AuthMethod, Config, GitHubAppConfig};

use crate::output;

const GITHUB_APP_SLUG_ENV: &str = "GHAD_GITHUB_APP_SLUG";
const GITHUB_APP_ID_ENV: &str = "GHAD_GITHUB_APP_ID";
const GITHUB_APP_PRIVATE_KEY_PATH_ENV: &str = "GHAD_GITHUB_APP_PRIVATE_KEY_PATH";
const GITHUB_APP_INSTALLATION_ID_ENV: &str = "GHAD_GITHUB_APP_INSTALLATION_ID";

/// Handle the `ghad auth configure` command.
///
/// Interactively configures GitHub App installation authentication.
pub async fn handle_configure() -> Result<()> {
    println!("🔐 GHAAD — Authentication Setup\n");
    configure_github_app_installation().await
}

/// Configure authentication via GitHub App installation.
async fn configure_github_app_installation() -> Result<()> {
    println!("\nA GitHub App installation lets GitHub prompt you to select account, organization, and repositories.");
    println!("The GitHub App needs repository permissions for Contents, Issues, Pull requests, and Metadata.\n");

    let app_slug = prompt_string("GitHub App slug", env_value(GITHUB_APP_SLUG_ENV))?;
    let app_id = prompt_u64("GitHub App ID", env_value(GITHUB_APP_ID_ENV))?;
    let private_key_path = prompt_string(
        "GitHub App private key path",
        env_value(GITHUB_APP_PRIVATE_KEY_PATH_ENV),
    )?;

    let install_url = format!("https://github.com/apps/{app_slug}/installations/new");
    match open_url_in_default_browser(&install_url) {
        Ok(()) => output::print_info("Opened GitHub App installation page in your default browser."),
        Err(err) => output::print_warning(&format!("Could not open browser automatically: {err}")),
    }
    output::print_info("Install the app for the account or organization you want, then choose all repositories or only selected repositories.");
    println!("{install_url}");

    let installation_id = match env_value(GITHUB_APP_INSTALLATION_ID_ENV) {
        Some(value) => value
            .parse::<u64>()
            .context("GHAD_GITHUB_APP_INSTALLATION_ID must be a positive integer")?,
        None => select_github_app_installation(app_id, &private_key_path).await?,
    };

    let mut config = ghad_core::config::load_default_config().unwrap_or_default();
    config.auth_method = AuthMethod::GitHubAppInstallation;
    config.github_token = None;
    config.github_app = Some(GitHubAppConfig {
        app_slug,
        app_id,
        private_key_path: private_key_path.into(),
        installation_id,
    });

    let sp = output::spinner("Validating GitHub App installation...");
    let client = match ghad_core::github::client::GithubClient::from_config(&config) {
        Ok(client) => client,
        Err(err) => {
            sp.finish_and_clear();
            return Err(err).context("Failed to create GitHub App installation client");
        }
    };
    if let Err(err) = ghad_core::github::repos::list_repos(&client).await {
        sp.finish_and_clear();
        return Err(err).context("Failed to validate GitHub App installation");
    }
    sp.finish_and_clear();

    save_config(config)?;

    let config_path = ghad_core::config::default_config_path()?;
    output::print_success("GitHub App installation authentication configured!");
    output::print_info(&format!("Configuration saved to {}", config_path.display()));

    Ok(())
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

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| trim_value(&value))
        .filter(|value| !value.is_empty())
}

fn prompt_string(prompt: &str, default: Option<String>) -> Result<String> {
    let mut input = Input::<String>::new().with_prompt(prompt.to_string());
    if let Some(default) = default {
        input = input.with_initial_text(default);
    }
    let value = input
        .interact_text()
        .context(format!("Failed to read {prompt}"))?;
    trim_required_prompt_value(prompt, &value)
}

fn trim_required_prompt_value(prompt: &str, value: &str) -> Result<String> {
    let value = trim_value(value);
    if value.is_empty() {
        anyhow::bail!("{prompt} cannot be empty");
    }
    Ok(value)
}

fn trim_value(value: &str) -> String {
    value.trim().to_string()
}

fn prompt_u64(prompt: &str, default: Option<String>) -> Result<u64> {
    let value = prompt_string(prompt, default)?;
    value
        .parse::<u64>()
        .context(format!("{prompt} must be a positive integer"))
}

async fn select_github_app_installation(app_id: u64, private_key_path: &str) -> Result<u64> {
    let _: String = Input::new()
        .with_prompt("Press Enter after completing the GitHub App installation in your browser")
        .allow_empty(true)
        .interact_text()
        .context("Failed to wait for installation confirmation")?;

    let sp = output::spinner("Finding GitHub App installations...");
    let installations =
        match list_github_app_installations(app_id, Path::new(private_key_path)).await {
            Ok(installations) => installations,
            Err(err) => {
                sp.finish_and_clear();
                return Err(err).context("Failed to list GitHub App installations");
            }
        };
    sp.finish_and_clear();

    if installations.is_empty() {
        anyhow::bail!(
            "No GitHub App installations found. Complete the installation in your browser, then run 'ghad auth configure' again."
        );
    }

    let labels: Vec<String> = installations
        .iter()
        .map(|installation| {
            let target = installation.target_type.as_deref().unwrap_or("account");
            let selection = installation
                .repository_selection
                .as_deref()
                .unwrap_or("repositories unknown");
            format!(
                "{} ({target}, {selection}, installation {})",
                installation.account_login, installation.id
            )
        })
        .collect();
    let selection = Select::new()
        .with_prompt("Select GitHub App installation")
        .items(&labels)
        .default(0)
        .interact()
        .context("Failed to select GitHub App installation")?;

    Ok(installations[selection].id)
}

fn save_config(config: Config) -> Result<()> {
    ghad_core::config::save_default_config(&config).context("Failed to save GitHub credentials")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_value_ignores_missing_vars() {
        assert!(env_value("GHAD_TEST_MISSING_ENV_VAR").is_none());
    }

    #[test]
    fn trim_required_prompt_value_trims_slug() {
        let value = trim_required_prompt_value("GitHub App slug", "  gh-agents-dispatcher \n")
            .unwrap();
        assert_eq!(value, "gh-agents-dispatcher");
    }

    #[test]
    fn trim_required_prompt_value_rejects_empty_after_trim() {
        let err = trim_required_prompt_value("GitHub App slug", " \n\t ").unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn prompt_u64_rejects_non_numeric_values() {
        let value = trim_required_prompt_value("GitHub App ID", " 12345 ").unwrap();
        assert_eq!(value.parse::<u64>().unwrap(), 12345);
        assert!("abc".parse::<u64>().is_err());
    }
}

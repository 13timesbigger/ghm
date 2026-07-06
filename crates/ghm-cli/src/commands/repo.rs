use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::github::repos;

use crate::output::{self, RepoRow};

/// Handle the `ghad repo list` command.
///
/// Lists repositories, optionally filtered by organization.
pub async fn handle_list(org: Option<String>) -> Result<()> {
    let config = load_default_config().context(
        "Failed to load configuration. Run 'ghad auth configure' first.",
    )?;

    let label = match &org {
        Some(name) => format!("Fetching repositories for '{}'...", name),
        None => "Fetching your repositories...".to_string(),
    };
    let sp = output::spinner(&label);

    let client = ghm_core::github::client::GithubClient::from_config(&config)?;
    let repos = match &org {
        Some(name) => repos::list_repos_by_org(&client, name).await
            .context(format!("Failed to fetch repos for org '{}'", name))?,
        None => repos::list_repos(&client).await
            .context("Failed to fetch user repositories")?,
    };

    sp.finish_and_clear();

    let rows: Vec<RepoRow> = repos
        .into_iter()
        .map(|repo| RepoRow {
            full_name: repo.full_name.clone(),
            description: repo.description.unwrap_or_default(),
            language: repo.default_branch.unwrap_or_else(|| "—".to_string()),
            stars: 0,
            updated_at: "—".to_string(),
        })
        .collect();

    output::print_repos_table(&rows);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

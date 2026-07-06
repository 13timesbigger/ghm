use anyhow::{Context, Result};
use std::collections::BTreeMap;

use ghad_core::config::load_default_config;
use ghad_core::github::{orgs, repos};
use ghad_core::models::GithubRepo;

use crate::output::{self, RepoRow};

/// Handle the `ghad repo list` command.
///
/// Lists repositories, optionally filtered by organization.
pub async fn handle_list(org: Option<String>, all_orgs: bool) -> Result<()> {
    let config = load_default_config()
        .context("Failed to load configuration. Run 'ghad auth configure' first.")?;

    let label = match (&org, all_orgs) {
        (Some(name), _) => format!("Fetching repositories for '{}'...", name),
        (None, true) => "Fetching repositories for all organizations...".to_string(),
        (None, false) => "Fetching your repositories...".to_string(),
    };
    let sp = output::spinner(&label);

    let client = ghad_core::github::client::GithubClient::from_config(&config)?;
    let (repos, warnings) = match (&org, all_orgs) {
        (Some(name), _) => {
            let repos = repos::list_repos_by_org(&client, name)
                .await
                .context(format!("Failed to fetch repos for org '{}'", name))?;
            (repos, Vec::new())
        }
        (None, true) => fetch_all_org_repos(&client).await?,
        (None, false) => {
            let repos = repos::list_repos(&client)
                .await
                .context("Failed to fetch user repositories")?;
            (repos, Vec::new())
        }
    };

    sp.finish_and_clear();

    for warning in warnings {
        output::print_warning(&warning);
    }

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

async fn fetch_all_org_repos(
    client: &ghad_core::github::client::GithubClient,
) -> Result<(Vec<GithubRepo>, Vec<String>)> {
    let orgs = orgs::list_orgs(client)
        .await
        .context("Failed to fetch organizations")?;
    let mut by_full_name = BTreeMap::new();
    let mut warnings = Vec::new();

    for org in orgs {
        match repos::list_repos_by_org(client, &org.login).await {
            Ok(repos) => {
                for repo in repos {
                    by_full_name.insert(repo.full_name.clone(), repo);
                }
            }
            Err(err) => warnings.push(format!("Skipped organization '{}': {}", org.login, err)),
        }
    }

    Ok((by_full_name.into_values().collect(), warnings))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

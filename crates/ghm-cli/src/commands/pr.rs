use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::github::pulls;

use crate::output::{self, PrRow};

/// Handle the `ghad pr list` command.
///
/// Lists pull requests, optionally filtered by repository and/or organization.
pub async fn handle_list(repo: Option<String>, org: Option<String>) -> Result<()> {
    let config = ghm_core::config::load_default_config().context(
        "Failed to load configuration. Run 'ghad auth configure' first.",
    )?;

    let label = "Fetching pull requests...".to_string();
    let sp = output::spinner(&label);

    let client = ghm_core::github::client::GithubClient::from_config(&config)?;

    let prs = match (repo, org) {
        (Some(r), Some(o)) => {
            let full_name = if r.contains('/') { r.clone() } else { format!("{}/{}", o, r) };
            let parts: Vec<&str> = full_name.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid repository format '{}'. Expected 'owner/repo'.", full_name);
            }
            ghm_core::github::pulls::list_pulls(&client, parts[0], parts[1])
                .await
                .context(format!("Failed to fetch PRs for '{}'", full_name))?
        }
        (Some(r), None) => {
            let parts: Vec<&str> = r.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid repository format '{}'. Expected 'owner/repo'.", r);
            }
            ghm_core::github::pulls::list_pulls(&client, parts[0], parts[1])
                .await
                .context(format!("Failed to fetch PRs for '{}'", r))?
        }
        (None, Some(o)) => {
            ghm_core::github::pulls::list_pulls_by_org(&client, &o)
                .await
                .context(format!("Failed to fetch PRs for org '{}'", o))?
        }
        (None, None) => {
            let config_dir = ghm_core::config::default_config_dir()?;
            let store = ghm_core::store::ObservedStore::new(&config_dir);
            let repos = store.load().unwrap_or_default();
            let mut all_prs = Vec::new();
            for r in repos.repositories {
                if r.watch_prs {
                    let parts: Vec<&str> = r.full_name.splitn(2, '/').collect();
                    if parts.len() == 2 {
                        if let Ok(p) = ghm_core::github::pulls::list_pulls(&client, parts[0], parts[1]).await {
                            all_prs.extend(p);
                        }
                    }
                }
            }
            all_prs
        }
    };

    sp.finish_and_clear();

    let rows: Vec<PrRow> = prs
        .into_iter()
        .map(|pr| PrRow {
            number: pr.number,
            title: pr.title,
            status: pr.state.clone(),
            author: pr.user_login,
            repo: pr.repo_full_name.clone(),
            created_at: pr.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();

    output::print_prs_table(&rows);

    Ok(())
}


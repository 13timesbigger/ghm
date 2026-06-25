use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::github::issues;

use crate::output::{self, IssueRow};

/// Resolve the full "owner/repo" name from optional --repo and --org flags.
fn resolve_repo(repo: &Option<String>, org: &Option<String>) -> Option<String> {
    match (repo, org) {
        (Some(r), Some(o)) => {
            if r.contains('/') {
                Some(r.clone())
            } else {
                Some(format!("{}/{}", o, r))
            }
        }
        (Some(r), None) => Some(r.clone()),
        _ => None,
    }
}

/// Handle the `ghm issue list` command.
///
/// Lists issues, optionally filtered by repository, organization, and/or project.
pub async fn handle_list(
    repo: Option<String>,
    org: Option<String>,
    _project: Option<String>,
) -> Result<()> {
    let config = ghm_core::config::load_default_config().context(
        "Failed to load configuration. Run 'ghm auth configure' first.",
    )?;

    let label = "Fetching issues...".to_string();
    let sp = output::spinner(&label);

    let client = ghm_core::github::client::GithubClient::from_config(&config)?;

    let issues = match (repo, org) {
        (Some(r), Some(o)) => {
            let full_name = if r.contains('/') { r.clone() } else { format!("{}/{}", o, r) };
            let parts: Vec<&str> = full_name.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid repository format '{}'. Expected 'owner/repo'.", full_name);
            }
            ghm_core::github::issues::list_issues(&client, parts[0], parts[1])
                .await
                .context(format!("Failed to fetch issues for '{}'", full_name))?
        }
        (Some(r), None) => {
            let parts: Vec<&str> = r.splitn(2, '/').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid repository format '{}'. Expected 'owner/repo'.", r);
            }
            ghm_core::github::issues::list_issues(&client, parts[0], parts[1])
                .await
                .context(format!("Failed to fetch issues for '{}'", r))?
        }
        (None, Some(o)) => {
            ghm_core::github::issues::list_issues_by_org(&client, &o)
                .await
                .context(format!("Failed to fetch issues for org '{}'", o))?
        }
        (None, None) => {
            let config_dir = ghm_core::config::default_config_dir()?;
            let store = ghm_core::store::ObservedStore::new(&config_dir);
            let repos = store.load().unwrap_or_default();
            let mut all_issues = Vec::new();
            for r in repos.repositories {
                if r.watch_issues {
                    let parts: Vec<&str> = r.full_name.splitn(2, '/').collect();
                    if parts.len() == 2 {
                        if let Ok(i) = ghm_core::github::issues::list_issues(&client, parts[0], parts[1]).await {
                            all_issues.extend(i);
                        }
                    }
                }
            }
            all_issues
        }
    };

    sp.finish_and_clear();

    let rows: Vec<IssueRow> = issues
        .into_iter()
        .map(|issue| IssueRow {
            number: issue.number,
            title: issue.title,
            status: issue.state.clone(),
            labels: issue.labels.join(", "),
            assignee: issue.user_login, // Using user_login as assignee proxy
            repo: issue.repo_full_name.clone(),
            created_at: issue.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();

    output::print_issues_table(&rows);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_repo_both() {
        let result = resolve_repo(
            &Some("myrepo".to_string()),
            &Some("myorg".to_string()),
        );
        assert_eq!(result, Some("myorg/myrepo".to_string()));
    }

    #[test]
    fn test_resolve_repo_full_name_with_org() {
        let result = resolve_repo(
            &Some("myorg/myrepo".to_string()),
            &Some("other".to_string()),
        );
        assert_eq!(result, Some("myorg/myrepo".to_string()));
    }

    #[test]
    fn test_resolve_repo_only_repo() {
        let result = resolve_repo(&Some("myorg/myrepo".to_string()), &None);
        assert_eq!(result, Some("myorg/myrepo".to_string()));
    }

    #[test]
    fn test_resolve_repo_neither() {
        let result = resolve_repo(&None, &None);
        assert!(result.is_none());
    }
}

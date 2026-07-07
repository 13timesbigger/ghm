use crate::error::{GhadError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubPullRequest;

/// List open pull requests for a specific repository.
pub async fn list_pulls(
    client: &GithubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<GithubPullRequest>> {
    let page = client
        .octocrab()
        .pulls(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to list PRs for {owner}/{repo}: {e}"),
        })?;

    let prs = client
        .octocrab()
        .all_pages(page)
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to paginate PRs for {owner}/{repo}: {e}"),
        })?;

    let full_name = format!("{owner}/{repo}");
    let prs = prs
        .into_iter()
        .map(|pr| GithubPullRequest {
            id: pr.id.into_inner(),
            number: pr.number,
            title: pr.title.unwrap_or_default(),
            state: pr
                .state
                .map(|s| format!("{s:?}").to_lowercase())
                .unwrap_or_else(|| "unknown".into()),
            html_url: pr
                .html_url
                .map(|u| u.to_string())
                .unwrap_or_default(),
            user_login: pr
                .user
                .map(|u| u.login)
                .unwrap_or_else(|| "unknown".into()),
            body: pr.body,
            created_at: pr.created_at.unwrap_or_default(),
            updated_at: pr.updated_at,
            repo_full_name: full_name.clone(),
        })
        .collect();
    Ok(prs)
}

/// List open PRs across all repos in an organisation.
pub async fn list_pulls_by_org(
    client: &GithubClient,
    org: &str,
) -> Result<Vec<GithubPullRequest>> {
    let repos = crate::github::repos::list_repos_by_org(client, org).await?;
    let mut all_prs = Vec::new();
    for repo in &repos {
        let parts: Vec<&str> = repo.full_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            match list_pulls(client, parts[0], parts[1]).await {
                Ok(prs) => all_prs.extend(prs),
                Err(e) => {
                    tracing::warn!("failed to list PRs for {}: {e}", repo.full_name);
                }
            }
        }
    }
    Ok(all_prs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn github_pr_model() {
        let pr = GithubPullRequest {
            id: 1,
            number: 42,
            title: "Fix bug".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/pull/42".into(),
            user_login: "dev".into(),
            body: Some("Fixes #10".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "o/r".into(),
        };
        assert_eq!(pr.number, 42);
        assert_eq!(pr.state, "open");
        assert_eq!(pr.repo_full_name, "o/r");
    }

    #[test]
    fn github_pr_serde_roundtrip() {
        let pr = GithubPullRequest {
            id: 99,
            number: 1,
            title: "PR".into(),
            state: "closed".into(),
            html_url: "https://github.com/a/b/pull/1".into(),
            user_login: "user".into(),
            body: None,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            repo_full_name: "a/b".into(),
        };
        let json = serde_json::to_string(&pr).unwrap();
        let back: GithubPullRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 99);
        assert_eq!(back.state, "closed");
    }

    #[test]
    fn github_pr_vec_serde() {
        let prs: Vec<GithubPullRequest> = vec![];
        let json = serde_json::to_string(&prs).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn pr_with_body() {
        let pr = GithubPullRequest {
            id: 1,
            number: 1,
            title: "t".into(),
            state: "open".into(),
            html_url: "h".into(),
            user_login: "u".into(),
            body: Some("long body text here".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "a/b".into(),
        };
        assert!(pr.body.is_some());
        assert!(pr.body.unwrap().contains("long body"));
    }
}

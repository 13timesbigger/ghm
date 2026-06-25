use crate::error::{GhmError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubIssue;

/// List open issues for a specific repository.
pub async fn list_issues(
    client: &GithubClient,
    owner: &str,
    repo: &str,
) -> Result<Vec<GithubIssue>> {
    let page = client
        .octocrab()
        .issues(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhmError::GitHubApi {
            message: format!("failed to list issues for {owner}/{repo}: {e}"),
        })?;

    let full_name = format!("{owner}/{repo}");
    let issues = page
        .items
        .into_iter()
        // Filter out pull requests (GitHub API returns them as issues too)
        .filter(|i| i.pull_request.is_none())
        .map(|i| GithubIssue {
            id: i.id.into_inner(),
            number: i.number,
            title: i.title,
            state: match i.state {
                octocrab::models::IssueState::Open => "open".to_string(),
                _ => "closed".to_string(),
            },
            html_url: i.html_url.to_string(),
            user_login: i.user.login,
            body: i.body,
            created_at: i.created_at,
            updated_at: Some(i.updated_at),
            repo_full_name: full_name.clone(),
            labels: i
                .labels
                .into_iter()
                .filter_map(|l| l.name.into())
                .collect(),
        })
        .collect();
    Ok(issues)
}

/// List open issues across all repos in an organisation.
pub async fn list_issues_by_org(
    client: &GithubClient,
    org: &str,
) -> Result<Vec<GithubIssue>> {
    let repos = crate::github::repos::list_repos_by_org(client, org).await?;
    let mut all_issues = Vec::new();
    for repo in &repos {
        let parts: Vec<&str> = repo.full_name.splitn(2, '/').collect();
        if parts.len() == 2 {
            match list_issues(client, parts[0], parts[1]).await {
                Ok(issues) => all_issues.extend(issues),
                Err(e) => {
                    tracing::warn!("failed to list issues for {}: {e}", repo.full_name);
                }
            }
        }
    }
    Ok(all_issues)
}

/// List issues for repos associated with a specific project.
/// Since Projects v2 doesn't easily map to repos, this takes explicit repo list.
pub async fn list_issues_for_repos(
    client: &GithubClient,
    repos: &[(String, String)],  // (owner, repo) pairs
) -> Result<Vec<GithubIssue>> {
    let mut all_issues = Vec::new();
    for (owner, repo) in repos {
        match list_issues(client, owner, repo).await {
            Ok(issues) => all_issues.extend(issues),
            Err(e) => {
                tracing::warn!("failed to list issues for {owner}/{repo}: {e}");
            }
        }
    }
    Ok(all_issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn github_issue_model() {
        let issue = GithubIssue {
            id: 1,
            number: 10,
            title: "Bug report".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/issues/10".into(),
            user_login: "reporter".into(),
            body: Some("Steps to reproduce...".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "o/r".into(),
            labels: vec!["bug".into()],
        };
        assert_eq!(issue.number, 10);
        assert_eq!(issue.labels.len(), 1);
    }

    #[test]
    fn github_issue_serde_roundtrip() {
        let issue = GithubIssue {
            id: 42,
            number: 5,
            title: "Feature".into(),
            state: "open".into(),
            html_url: "https://github.com/a/b/issues/5".into(),
            user_login: "u".into(),
            body: None,
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            repo_full_name: "a/b".into(),
            labels: vec!["enhancement".into(), "good-first-issue".into()],
        };
        let json = serde_json::to_string(&issue).unwrap();
        let back: GithubIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.labels.len(), 2);
    }

    #[test]
    fn github_issue_empty_labels() {
        let issue = GithubIssue {
            id: 1,
            number: 1,
            title: "t".into(),
            state: "open".into(),
            html_url: "h".into(),
            user_login: "u".into(),
            body: None,
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "a/b".into(),
            labels: vec![],
        };
        assert!(issue.labels.is_empty());
    }

    #[test]
    fn github_issue_vec_serde() {
        let issues: Vec<GithubIssue> = vec![];
        let json = serde_json::to_string(&issues).unwrap();
        assert_eq!(json, "[]");
    }

    #[test]
    fn github_issue_with_multiple_labels() {
        let issue = GithubIssue {
            id: 2,
            number: 2,
            title: "Multi-label".into(),
            state: "closed".into(),
            html_url: "https://github.com/x/y/issues/2".into(),
            user_login: "dev".into(),
            body: Some("body".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "x/y".into(),
            labels: vec!["bug".into(), "p1".into(), "regression".into()],
        };
        assert_eq!(issue.labels.len(), 3);
        assert!(issue.labels.contains(&"regression".to_string()));
    }
}

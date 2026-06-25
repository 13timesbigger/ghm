use crate::error::{GhmError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubRepo;
use octocrab::params;

/// List all repositories accessible to the authenticated user.
pub async fn list_repos(client: &GithubClient) -> Result<Vec<GithubRepo>> {
    let page = client
        .octocrab()
        .current()
        .list_repos_for_authenticated_user()
        .sort("updated")
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhmError::GitHubApi {
            message: format!("failed to list repos: {e}"),
        })?;

    Ok(map_repos(page.items))
}

/// List repositories belonging to a specific organisation.
pub async fn list_repos_by_org(client: &GithubClient, org: &str) -> Result<Vec<GithubRepo>> {
    let page = client
        .octocrab()
        .orgs(org)
        .list_repos()
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhmError::GitHubApi {
            message: format!("failed to list repos for org {org}: {e}"),
        })?;

    Ok(map_repos(page.items))
}

fn map_repos(repos: Vec<octocrab::models::Repository>) -> Vec<GithubRepo> {
    repos
        .into_iter()
        .map(|r| {
            GithubRepo {
                full_name: r
                    .full_name
                    .unwrap_or_default(),
                id: r.id.into_inner(),
                description: r.description,
                html_url: r
                    .html_url
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                private: r.private.unwrap_or(false),
                default_branch: r.default_branch,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_repo_model() {
        let repo = GithubRepo {
            full_name: "org/repo".into(),
            id: 42,
            description: Some("A repo".into()),
            html_url: "https://github.com/org/repo".into(),
            private: false,
            default_branch: Some("main".into()),
        };
        assert_eq!(repo.full_name, "org/repo");
        assert!(!repo.private);
    }

    #[test]
    fn github_repo_serde_roundtrip() {
        let repo = GithubRepo {
            full_name: "a/b".into(),
            id: 1,
            description: None,
            html_url: "https://github.com/a/b".into(),
            private: true,
            default_branch: None,
        };
        let json = serde_json::to_string(&repo).unwrap();
        let back: GithubRepo = serde_json::from_str(&json).unwrap();
        assert_eq!(back, repo);
    }

    #[test]
    fn github_repo_vec_serde() {
        let repos = vec![
            GithubRepo {
                full_name: "x/y".into(),
                id: 10,
                description: None,
                html_url: "https://github.com/x/y".into(),
                private: false,
                default_branch: Some("main".into()),
            },
        ];
        let json = serde_json::to_string(&repos).unwrap();
        let back: Vec<GithubRepo> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 1);
    }

    #[test]
    fn map_repos_empty() {
        let result = map_repos(vec![]);
        assert!(result.is_empty());
    }
}

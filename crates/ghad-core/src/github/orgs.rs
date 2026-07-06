use crate::error::{GhadError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubOrg;
use std::collections::BTreeMap;

/// List organisations the authenticated user belongs to.
pub async fn list_orgs(client: &GithubClient) -> Result<Vec<GithubOrg>> {
    let page = client
        .octocrab()
        .current()
        .list_org_memberships_for_authenticated_user()
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to list orgs: {e}"),
        })?;

    let memberships =
        client
            .octocrab()
            .all_pages(page)
            .await
            .map_err(|e| GhadError::GitHubApi {
                message: format!("failed to paginate orgs: {e}"),
            })?;

    let orgs: Vec<GithubOrg> = memberships
        .into_iter()
        .map(|m| {
            let org = m.organization;
            GithubOrg {
                login: org.login,
                id: org.id.into_inner(),
                description: org.description,
                avatar_url: Some(org.avatar_url.to_string()),
            }
        })
        .collect();

    if orgs.is_empty() {
        return list_orgs_from_accessible_repos(client).await;
    }

    Ok(orgs)
}

async fn list_orgs_from_accessible_repos(client: &GithubClient) -> Result<Vec<GithubOrg>> {
    let current_user = client
        .octocrab()
        .current()
        .user()
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to fetch current user: {e}"),
        })?;
    let repos = crate::github::repos::list_repos(client).await?;
    let mut orgs = BTreeMap::new();

    for repo in repos {
        let Some((owner, _)) = repo.full_name.split_once('/') else {
            continue;
        };
        if owner.eq_ignore_ascii_case(&current_user.login) {
            continue;
        }
        orgs.entry(owner.to_string()).or_insert_with(|| GithubOrg {
            login: owner.to_string(),
            id: 0,
            description: Some("Inferred from accessible repositories".to_string()),
            avatar_url: None,
        });
    }

    Ok(orgs.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_org_model_fields() {
        let org = GithubOrg {
            login: "testorg".into(),
            id: 1,
            description: Some("desc".into()),
            avatar_url: Some("https://example.com/avatar.png".into()),
        };
        assert_eq!(org.login, "testorg");
        assert_eq!(org.id, 1);
        assert_eq!(org.description, Some("desc".into()));
    }

    #[test]
    fn github_org_serde_roundtrip() {
        let org = GithubOrg {
            login: "org".into(),
            id: 42,
            description: None,
            avatar_url: None,
        };
        let json = serde_json::to_string(&org).unwrap();
        let back: GithubOrg = serde_json::from_str(&json).unwrap();
        assert_eq!(back, org);
    }

    #[test]
    fn github_org_vec_serde() {
        let orgs = vec![
            GithubOrg {
                login: "a".into(),
                id: 1,
                description: None,
                avatar_url: None,
            },
            GithubOrg {
                login: "b".into(),
                id: 2,
                description: Some("B org".into()),
                avatar_url: Some("https://img.com/b.png".into()),
            },
        ];
        let json = serde_json::to_string(&orgs).unwrap();
        let back: Vec<GithubOrg> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].login, "b");
    }
}

use crate::error::{GhmError, Result};
use crate::github::client::GithubClient;
use crate::models::GithubOrg;

/// List organisations the authenticated user belongs to.
pub async fn list_orgs(client: &GithubClient) -> Result<Vec<GithubOrg>> {
    let page = client
        .octocrab()
        .current()
        .list_org_memberships_for_authenticated_user()
        .send()
        .await
        .map_err(|e| GhmError::GitHubApi {
            message: format!("failed to list orgs: {e}"),
        })?;

    let orgs = page
        .items
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
    Ok(orgs)
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

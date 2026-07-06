use crate::error::{GhadError, Result};

/// A thin wrapper around `octocrab::Octocrab` that constructs the client
/// from a personal access token.
#[derive(Clone)]
pub struct GithubClient {
    inner: octocrab::Octocrab,
    token: String,
}

impl std::fmt::Debug for GithubClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GithubClient")
            .field("token", &"[REDACTED]")
            .finish()
    }
}

impl GithubClient {
    /// Create a new client with the given PAT.
    pub fn new(token: &str) -> Result<Self> {
        let octocrab = octocrab::Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .map_err(|e| GhadError::GitHubApi {
                message: format!("failed to build Octocrab client: {e}"),
            })?;
        Ok(Self {
            inner: octocrab,
            token: token.to_string(),
        })
    }

    /// Create a client from a `Config`, pulling the token from it.
    pub fn from_config(config: &crate::models::Config) -> Result<Self> {
        let token = config
            .github_token
            .as_deref()
            .ok_or(GhadError::TokenMissing)?;
        Self::new(token)
    }

    /// Get a reference to the inner Octocrab instance.
    pub fn octocrab(&self) -> &octocrab::Octocrab {
        &self.inner
    }

    /// Get the token (useful for raw HTTP calls).
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Make a raw GraphQL query and return the JSON response.
    pub async fn graphql<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<T> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "query": query,
            "variables": variables,
        });
        let resp = client
            .post("https://api.github.com/graphql")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "ghad-core")
            .json(&body)
            .send()
            .await
            .map_err(|e| GhadError::GraphQL {
                message: e.to_string(),
            })?;

        if !resp.status().is_success() {
            return Err(GhadError::GraphQL {
                message: format!("GraphQL request failed with status {}", resp.status()),
            });
        }

        let result: T = resp.json().await.map_err(|e| GhadError::GraphQL {
            message: e.to_string(),
        })?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Config;

    #[tokio::test]
    async fn new_client_ok() {
        let client = GithubClient::new("ghp_testtoken123").unwrap();
        assert_eq!(client.token(), "ghp_testtoken123");
    }

    #[tokio::test]
    async fn debug_redacts_token() {
        let client = GithubClient::new("ghp_secret").unwrap();
        let debug = format!("{:?}", client);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("ghp_secret"));
    }

    #[tokio::test]
    async fn from_config_ok() {
        let config = Config {
            github_token: Some("ghp_abc".into()),
            ..Default::default()
        };
        let client = GithubClient::from_config(&config).unwrap();
        assert_eq!(client.token(), "ghp_abc");
    }

    #[tokio::test]
    async fn from_config_missing_token() {
        let config = Config::default();
        let err = GithubClient::from_config(&config).unwrap_err();
        assert!(err.to_string().contains("not configured"));
    }

    #[tokio::test]
    async fn octocrab_ref_accessible() {
        let client = GithubClient::new("ghp_test").unwrap();
        // Just assert we can call it without panic
        let _oct = client.octocrab();
    }

    #[tokio::test]
    async fn client_is_clone() {
        let c1 = GithubClient::new("ghp_test").unwrap();
        let c2 = c1.clone();
        assert_eq!(c2.token(), "ghp_test");
    }
}

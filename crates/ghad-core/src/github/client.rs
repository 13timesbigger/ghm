use crate::error::{GhadError, Result};
use crate::models::{AuthMethod, Config, GitHubAppInstallation};
use std::path::Path;

/// A thin wrapper around `octocrab::Octocrab` that tracks the configured auth mode.
#[derive(Clone)]
pub struct GithubClient {
    inner: octocrab::Octocrab,
    token: String,
    auth_method: AuthMethod,
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
            auth_method: AuthMethod::PersonalAccessToken,
        })
    }

    /// Create a client from a `Config`, pulling the token from it.
    pub fn from_config(config: &Config) -> Result<Self> {
        match config.auth_method {
            AuthMethod::GitHubAppInstallation => Self::from_github_app_config(config),
            AuthMethod::PersonalAccessToken | AuthMethod::DeviceFlow => {
                let token = config
                    .github_token
                    .as_deref()
                    .ok_or(GhadError::TokenMissing)?;
                Self::new(token)
            }
        }
    }

    fn from_github_app_config(config: &Config) -> Result<Self> {
        let app = config
            .github_app
            .as_ref()
            .ok_or_else(|| GhadError::ConfigInvalid {
                message: "GitHub App installation auth is selected, but github_app config is missing".into(),
            })?;
        let app_client = build_github_app_client(app.app_id, &app.private_key_path)?;
        let installation_client = app_client
            .installation(octocrab::models::InstallationId(app.installation_id))
            .map_err(|e| GhadError::GitHubApi {
                message: format!("failed to build GitHub App installation client: {e}"),
            })?;

        Ok(Self {
            inner: installation_client,
            token: String::new(),
            auth_method: AuthMethod::GitHubAppInstallation,
        })
    }

    /// Get a reference to the inner Octocrab instance.
    pub fn octocrab(&self) -> &octocrab::Octocrab {
        &self.inner
    }

    /// Get the token (useful for raw HTTP calls).
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Whether the client uses GitHub App installation authentication.
    pub fn is_github_app_installation(&self) -> bool {
        self.auth_method == AuthMethod::GitHubAppInstallation
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

/// Build an Octocrab client authenticated as a GitHub App.
pub fn build_github_app_client(app_id: u64, private_key_path: &Path) -> Result<octocrab::Octocrab> {
    let private_key =
        std::fs::read(private_key_path).map_err(|e| GhadError::ConfigRead { source: e })?;
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(&private_key).map_err(|e| {
        GhadError::ConfigInvalid {
            message: format!(
                "failed to read GitHub App private key '{}': {e}",
                private_key_path.display()
            ),
        }
    })?;
    octocrab::Octocrab::builder()
        .app(octocrab::models::AppId(app_id), key)
        .build()
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to build GitHub App client: {e}"),
        })
}

/// List installations for the authenticated GitHub App.
pub async fn list_github_app_installations(
    app_id: u64,
    private_key_path: &Path,
) -> Result<Vec<GitHubAppInstallation>> {
    let app_client = build_github_app_client(app_id, private_key_path)?;
    let page = app_client
        .apps()
        .installations()
        .per_page(100)
        .send()
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to list GitHub App installations: {e}"),
        })?;
    let installations = app_client
        .all_pages(page)
        .await
        .map_err(|e| GhadError::GitHubApi {
            message: format!("failed to paginate GitHub App installations: {e}"),
        })?;

    Ok(installations
        .into_iter()
        .map(|installation| GitHubAppInstallation {
            id: installation.id.into_inner(),
            account_login: installation.account.login,
            target_type: installation.target_type,
            repository_selection: installation.repository_selection,
        })
        .collect())
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
            auth_method: AuthMethod::PersonalAccessToken,
            ..Default::default()
        };
        let client = GithubClient::from_config(&config).unwrap();
        assert_eq!(client.token(), "ghp_abc");
    }

    #[tokio::test]
    async fn from_config_missing_github_app_config() {
        let config = Config::default();
        let err = GithubClient::from_config(&config).unwrap_err();
        assert!(err.to_string().contains("github_app config is missing"));
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

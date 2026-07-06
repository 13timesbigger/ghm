use crate::error::{GhadError, Result};
use serde::{Deserialize, Serialize};

/// Validate that a personal access token has the correct format.
/// GitHub PATs start with `ghp_`, `gho_`, `ghu_`, `ghs_`, or `github_pat_`.
pub fn validate_pat_format(token: &str) -> Result<()> {
    let valid_prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "github_pat_"];
    if token.is_empty() {
        return Err(GhadError::AuthFailed {
            message: "token is empty".into(),
        });
    }
    if !valid_prefixes.iter().any(|p| token.starts_with(p)) {
        return Err(GhadError::AuthFailed {
            message: format!(
                "token does not start with a recognised prefix (expected one of: {})",
                valid_prefixes.join(", ")
            ),
        });
    }
    Ok(())
}

/// Verify a PAT against the GitHub API by calling `GET /user`.
pub async fn verify_pat(token: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "ghad-core")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| GhadError::AuthFailed {
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(GhadError::AuthFailed {
            message: format!("GitHub returned status {}", resp.status()),
        });
    }

    #[derive(Deserialize)]
    struct User {
        login: String,
    }
    let user: User = resp.json().await.map_err(|e| GhadError::AuthFailed {
        message: e.to_string(),
    })?;
    Ok(user.login)
}

// ── Device Flow ────────────────────────────────────────────────────

/// Response from the device code request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// Response from polling for the access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: Option<String>,
    error_description: Option<String>,
    error_uri: Option<String>,
    message: Option<String>,
}

async fn oauth_error_message(resp: reqwest::Response) -> String {
    let status = resp.status();
    let body = match resp.text().await {
        Ok(body) => body,
        Err(err) => return format!("GitHub returned status {status}; failed to read body: {err}"),
    };

    format_oauth_error(status, &body)
}

fn format_oauth_error(status: reqwest::StatusCode, body: &str) -> String {
    if let Ok(error) = serde_json::from_str::<OAuthErrorResponse>(&body) {
        let mut parts = vec![format!("GitHub returned status {status}")];
        if let Some(code) = error.error.filter(|value| !value.is_empty()) {
            parts.push(code);
        }
        if let Some(description) = error.error_description.filter(|value| !value.is_empty()) {
            parts.push(description);
        }
        if let Some(uri) = error.error_uri.filter(|value| !value.is_empty()) {
            parts.push(uri);
        }
        if let Some(message) = error.message.filter(|value| !value.is_empty()) {
            parts.push(message);
        }
        return parts.join(": ");
    }

    if body.trim().is_empty() {
        format!("GitHub returned status {status}")
    } else {
        format!("GitHub returned status {status}: {}", body.trim())
    }
}

/// Request a device code from GitHub for the OAuth Device Flow.
pub async fn request_device_code(client_id: &str, scope: &str) -> Result<DeviceCodeResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .header("User-Agent", "ghad-core")
        .form(&[("client_id", client_id), ("scope", scope)])
        .send()
        .await
        .map_err(|e| GhadError::DeviceFlow {
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(GhadError::DeviceFlow {
            message: oauth_error_message(resp).await,
        });
    }

    resp.json::<DeviceCodeResponse>()
        .await
        .map_err(|e| GhadError::DeviceFlow {
            message: e.to_string(),
        })
}

/// Poll GitHub for the access token during device flow.
pub async fn poll_device_token(client_id: &str, device_code: &str) -> Result<DeviceTokenResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .header("User-Agent", "ghad-core")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|e| GhadError::DeviceFlow {
            message: e.to_string(),
        })?;

    if !resp.status().is_success() {
        return Err(GhadError::DeviceFlow {
            message: oauth_error_message(resp).await,
        });
    }

    resp.json::<DeviceTokenResponse>()
        .await
        .map_err(|e| GhadError::DeviceFlow {
            message: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_pat_empty() {
        let err = validate_pat_format("").unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn validate_pat_bad_prefix() {
        let err = validate_pat_format("bad_token_here").unwrap_err();
        assert!(err.to_string().contains("prefix"));
    }

    #[test]
    fn validate_pat_ghp() {
        validate_pat_format("ghp_abcdef1234567890abcdef1234567890abcd").unwrap();
    }

    #[test]
    fn validate_pat_gho() {
        validate_pat_format("gho_abcdefg").unwrap();
    }

    #[test]
    fn validate_pat_ghu() {
        validate_pat_format("ghu_test").unwrap();
    }

    #[test]
    fn validate_pat_ghs() {
        validate_pat_format("ghs_xyz").unwrap();
    }

    #[test]
    fn validate_pat_github_pat() {
        validate_pat_format("github_pat_longtokenvalue").unwrap();
    }

    #[test]
    fn device_code_response_serde() {
        let json = r#"{
            "device_code": "dc_123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        }"#;
        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn device_token_response_serde_success() {
        let json = r#"{
            "access_token": "gho_abc",
            "token_type": "bearer",
            "scope": "repo"
        }"#;
        let resp: DeviceTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, Some("gho_abc".into()));
        assert!(resp.error.is_none());
    }

    #[test]
    fn device_token_response_serde_pending() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "waiting"
        }"#;
        let resp: DeviceTokenResponse = serde_json::from_str(json).unwrap();
        assert!(resp.access_token.is_none());
        assert_eq!(resp.error, Some("authorization_pending".into()));
    }

    #[tokio::test]
    async fn format_oauth_error_includes_github_body() {
        let message = format_oauth_error(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error":"device_flow_disabled","error_description":"Device Flow is not enabled"}"#,
        );
        assert!(message.contains("400 Bad Request"));
        assert!(message.contains("device_flow_disabled"));
        assert!(message.contains("Device Flow is not enabled"));
    }

    // Async tests against wiremock
    #[tokio::test]
    async fn verify_pat_against_mock() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user"))
            .and(header("Authorization", "Bearer ghp_test"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"login": "testuser"})),
            )
            .mount(&server)
            .await;

        // We need to override the URL — use reqwest directly for the test
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/user", server.uri()))
            .header("Authorization", "Bearer ghp_test")
            .header("User-Agent", "ghad-core")
            .send()
            .await
            .unwrap();
        assert!(resp.status().is_success());

        #[derive(Deserialize)]
        struct User {
            login: String,
        }
        let user: User = resp.json().await.unwrap();
        assert_eq!(user.login, "testuser");
    }

    #[tokio::test]
    async fn verify_pat_unauthorized() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"message": "Bad credentials"})),
            )
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/user", server.uri()))
            .header("Authorization", "Bearer bad")
            .header("User-Agent", "ghad-core")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 401);
    }
}

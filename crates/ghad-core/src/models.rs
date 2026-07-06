use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Auth ───────────────────────────────────────────────────────────

/// How the user authenticated with GitHub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    PersonalAccessToken,
    DeviceFlow,
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::PersonalAccessToken
    }
}

// ── Agent ──────────────────────────────────────────────────────────

/// Supported AI coding agents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Codex,
    Agy,
    Claude,
    Copilot,
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentType::Codex => write!(f, "codex"),
            AgentType::Agy => write!(f, "agy"),
            AgentType::Claude => write!(f, "claude"),
            AgentType::Copilot => write!(f, "copilot"),
        }
    }
}

/// Paths to agent CLI binaries (optional overrides).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AgentPaths {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codex: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agy: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copilot: Option<PathBuf>,
}

// ── Config ─────────────────────────────────────────────────────────

/// Application configuration stored in `~/.config/ghad/config.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github_token: Option<String>,

    #[serde(default)]
    pub auth_method: AuthMethod,

    #[serde(default = "default_poll_interval")]
    pub default_poll_interval_secs: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_working_dir: Option<PathBuf>,

    #[serde(default)]
    pub agent_paths: AgentPaths,
}

fn default_poll_interval() -> u64 {
    30
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: None,
            auth_method: AuthMethod::default(),
            default_poll_interval_secs: default_poll_interval(),
            default_working_dir: None,
            agent_paths: AgentPaths::default(),
        }
    }
}

// ── Observed repos ─────────────────────────────────────────────────

/// A single repository being monitored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservedRepo {
    pub full_name: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub watch_issues: bool,
    #[serde(default = "default_true")]
    pub watch_prs: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval_secs: Option<u64>,
    pub added_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

/// Wrapper for the `observed.json` file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ObservedRepos {
    #[serde(default)]
    pub repositories: Vec<ObservedRepo>,
}

// ── Prompts ────────────────────────────────────────────────────────

/// Prompt templates for a single context (issues / PRs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PromptConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_prompt: Option<String>,
}

/// Repo-specific prompt overrides.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RepoPrompt {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_prompt: Option<String>,
}

/// Top-level prompts file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Prompts {
    #[serde(default)]
    pub global: PromptConfig,
    #[serde(default)]
    pub repos: HashMap<String, RepoPrompt>,
}

// ── Daemon state ───────────────────────────────────────────────────

/// Per-repo tracking of already-seen event IDs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SeenEvents {
    #[serde(default)]
    pub issue_ids: Vec<u64>,
    #[serde(default)]
    pub pr_ids: Vec<u64>,
}

/// Per-repo state entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RepoSeenState {
    #[serde(default)]
    pub seen: SeenEvents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_poll: Option<DateTime<Utc>>,
}

/// Daemon runtime state persisted in `state.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DaemonState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon_pid: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon_status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_poll: Option<DateTime<Utc>>,

    #[serde(default)]
    pub repos: HashMap<String, RepoSeenState>,
}

// ── GitHub data models ─────────────────────────────────────────────

/// A GitHub organization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GithubOrg {
    pub login: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// A GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GithubRepo {
    pub full_name: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub html_url: String,
    #[serde(default)]
    pub private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

/// A GitHub Projects v2 project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GithubProject {
    pub id: String,
    pub title: String,
    pub number: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    pub closed: bool,
    pub url: String,
}

/// A GitHub pull request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GithubPullRequest {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub user_login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub repo_full_name: String,
}

/// A GitHub issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GithubIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub user_login: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    pub repo_full_name: String,
    #[serde(default)]
    pub labels: Vec<String>,
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── AuthMethod ─────────────────────────────────────────────────

    #[test]
    fn auth_method_default_is_pat() {
        assert_eq!(AuthMethod::default(), AuthMethod::PersonalAccessToken);
    }

    #[test]
    fn auth_method_serde_roundtrip() {
        let pat = AuthMethod::PersonalAccessToken;
        let json = serde_json::to_string(&pat).unwrap();
        assert_eq!(json, "\"personal_access_token\"");
        let back: AuthMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pat);

        let df = AuthMethod::DeviceFlow;
        let json = serde_json::to_string(&df).unwrap();
        assert_eq!(json, "\"device_flow\"");
    }

    // ── AgentType ──────────────────────────────────────────────────

    #[test]
    fn agent_type_display() {
        assert_eq!(AgentType::Codex.to_string(), "codex");
        assert_eq!(AgentType::Agy.to_string(), "agy");
        assert_eq!(AgentType::Claude.to_string(), "claude");
        assert_eq!(AgentType::Copilot.to_string(), "copilot");
    }

    #[test]
    fn agent_type_serde_roundtrip() {
        for agent in &[
            AgentType::Codex,
            AgentType::Agy,
            AgentType::Claude,
            AgentType::Copilot,
        ] {
            let json = serde_json::to_string(agent).unwrap();
            let back: AgentType = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, agent);
        }
    }

    #[test]
    fn agent_type_eq_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(AgentType::Codex);
        set.insert(AgentType::Codex);
        assert_eq!(set.len(), 1);
    }

    // ── AgentPaths ─────────────────────────────────────────────────

    #[test]
    fn agent_paths_default_all_none() {
        let ap = AgentPaths::default();
        assert!(ap.codex.is_none());
        assert!(ap.agy.is_none());
        assert!(ap.claude.is_none());
        assert!(ap.copilot.is_none());
    }

    #[test]
    fn agent_paths_serde_skip_none() {
        let ap = AgentPaths::default();
        let json = serde_json::to_string(&ap).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn agent_paths_serde_with_values() {
        let ap = AgentPaths {
            codex: Some(PathBuf::from("/usr/local/bin/codex")),
            ..Default::default()
        };
        let json = serde_json::to_string(&ap).unwrap();
        assert!(json.contains("codex"));
        let back: AgentPaths = serde_json::from_str(&json).unwrap();
        assert_eq!(back.codex, Some(PathBuf::from("/usr/local/bin/codex")));
    }

    // ── Config ─────────────────────────────────────────────────────

    #[test]
    fn config_default() {
        let c = Config::default();
        assert!(c.github_token.is_none());
        assert_eq!(c.auth_method, AuthMethod::PersonalAccessToken);
        assert_eq!(c.default_poll_interval_secs, 30);
        assert!(c.default_working_dir.is_none());
    }

    #[test]
    fn config_serde_roundtrip() {
        let c = Config {
            github_token: Some("ghp_test123".into()),
            auth_method: AuthMethod::DeviceFlow,
            default_poll_interval_secs: 60,
            default_working_dir: Some(PathBuf::from("/tmp/work")),
            agent_paths: AgentPaths {
                codex: Some(PathBuf::from("/bin/codex")),
                ..Default::default()
            },
        };
        let json = serde_json::to_string_pretty(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn config_deserialize_minimal() {
        let json = r#"{}"#;
        let c: Config = serde_json::from_str(json).unwrap();
        assert_eq!(c.default_poll_interval_secs, 30);
        assert_eq!(c.auth_method, AuthMethod::PersonalAccessToken);
    }

    // ── ObservedRepo ───────────────────────────────────────────────

    #[test]
    fn observed_repo_serde() {
        let repo = ObservedRepo {
            full_name: "owner/repo".into(),
            url: "https://github.com/owner/repo".into(),
            watch_issues: true,
            watch_prs: false,
            agent: Some(AgentType::Claude),
            prompt: Some("fix bugs".into()),
            poll_interval_secs: Some(60),
            added_at: Utc::now(),
        };
        let json = serde_json::to_string(&repo).unwrap();
        let back: ObservedRepo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.full_name, "owner/repo");
        assert!(!back.watch_prs);
        assert_eq!(back.agent, Some(AgentType::Claude));
    }

    #[test]
    fn observed_repo_defaults() {
        let json = r#"{
            "full_name": "a/b",
            "url": "https://github.com/a/b",
            "added_at": "2025-01-01T00:00:00Z"
        }"#;
        let repo: ObservedRepo = serde_json::from_str(json).unwrap();
        assert!(repo.watch_issues);
        assert!(repo.watch_prs);
        assert!(repo.agent.is_none());
    }

    // ── ObservedRepos ──────────────────────────────────────────────

    #[test]
    fn observed_repos_default_empty() {
        let or = ObservedRepos::default();
        assert!(or.repositories.is_empty());
    }

    #[test]
    fn observed_repos_serde() {
        let repos = ObservedRepos {
            repositories: vec![ObservedRepo {
                full_name: "x/y".into(),
                url: "https://github.com/x/y".into(),
                watch_issues: true,
                watch_prs: true,
                agent: None,
                prompt: None,
                poll_interval_secs: None,
                added_at: Utc::now(),
            }],
        };
        let json = serde_json::to_string(&repos).unwrap();
        let back: ObservedRepos = serde_json::from_str(&json).unwrap();
        assert_eq!(back.repositories.len(), 1);
    }

    // ── PromptConfig ───────────────────────────────────────────────

    #[test]
    fn prompt_config_default() {
        let pc = PromptConfig::default();
        assert!(pc.issue_prompt.is_none());
        assert!(pc.pr_prompt.is_none());
    }

    #[test]
    fn prompt_config_serde() {
        let pc = PromptConfig {
            issue_prompt: Some("Review this issue".into()),
            pr_prompt: None,
        };
        let json = serde_json::to_string(&pc).unwrap();
        let back: PromptConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.issue_prompt, Some("Review this issue".into()));
        assert!(back.pr_prompt.is_none());
    }

    // ── Prompts ────────────────────────────────────────────────────

    #[test]
    fn prompts_default() {
        let p = Prompts::default();
        assert!(p.repos.is_empty());
    }

    #[test]
    fn prompts_serde_roundtrip() {
        let mut repos = HashMap::new();
        repos.insert(
            "owner/repo".to_string(),
            RepoPrompt {
                issue_prompt: Some("handle issue".into()),
                pr_prompt: Some("review pr".into()),
            },
        );
        let p = Prompts {
            global: PromptConfig {
                issue_prompt: Some("global issue".into()),
                pr_prompt: None,
            },
            repos,
        };
        let json = serde_json::to_string_pretty(&p).unwrap();
        let back: Prompts = serde_json::from_str(&json).unwrap();
        assert_eq!(back.global.issue_prompt, Some("global issue".into()));
        assert!(back.repos.contains_key("owner/repo"));
    }

    // ── SeenEvents ─────────────────────────────────────────────────

    #[test]
    fn seen_events_default() {
        let se = SeenEvents::default();
        assert!(se.issue_ids.is_empty());
        assert!(se.pr_ids.is_empty());
    }

    // ── RepoSeenState ──────────────────────────────────────────────

    #[test]
    fn repo_seen_state_default() {
        let rss = RepoSeenState::default();
        assert!(rss.last_poll.is_none());
        assert!(rss.seen.issue_ids.is_empty());
    }

    // ── DaemonState ────────────────────────────────────────────────

    #[test]
    fn daemon_state_default() {
        let ds = DaemonState::default();
        assert!(ds.daemon_pid.is_none());
        assert!(ds.daemon_status.is_none());
        assert!(ds.repos.is_empty());
    }

    #[test]
    fn daemon_state_serde_roundtrip() {
        let mut repos = HashMap::new();
        repos.insert(
            "a/b".to_string(),
            RepoSeenState {
                seen: SeenEvents {
                    issue_ids: vec![1, 2, 3],
                    pr_ids: vec![10],
                },
                last_poll: Some(Utc::now()),
            },
        );
        let ds = DaemonState {
            daemon_pid: Some(999),
            daemon_status: Some("running".into()),
            last_poll: Some(Utc::now()),
            repos,
        };
        let json = serde_json::to_string_pretty(&ds).unwrap();
        let back: DaemonState = serde_json::from_str(&json).unwrap();
        assert_eq!(back.daemon_pid, Some(999));
        assert_eq!(back.repos["a/b"].seen.issue_ids, vec![1, 2, 3]);
    }

    // ── GithubOrg ──────────────────────────────────────────────────

    #[test]
    fn github_org_serde() {
        let org = GithubOrg {
            login: "myorg".into(),
            id: 42,
            description: Some("My Org".into()),
            avatar_url: None,
        };
        let json = serde_json::to_string(&org).unwrap();
        let back: GithubOrg = serde_json::from_str(&json).unwrap();
        assert_eq!(back.login, "myorg");
        assert_eq!(back.id, 42);
    }

    // ── GithubRepo ─────────────────────────────────────────────────

    #[test]
    fn github_repo_serde() {
        let repo = GithubRepo {
            full_name: "org/repo".into(),
            id: 100,
            description: Some("desc".into()),
            html_url: "https://github.com/org/repo".into(),
            private: true,
            default_branch: Some("main".into()),
        };
        let json = serde_json::to_string(&repo).unwrap();
        let back: GithubRepo = serde_json::from_str(&json).unwrap();
        assert!(back.private);
        assert_eq!(back.default_branch, Some("main".into()));
    }

    // ── GithubProject ──────────────────────────────────────────────

    #[test]
    fn github_project_serde() {
        let p = GithubProject {
            id: "PVT_abc".into(),
            title: "My Project".into(),
            number: 1,
            short_description: Some("desc".into()),
            closed: false,
            url: "https://github.com/orgs/o/projects/1".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: GithubProject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.title, "My Project");
        assert!(!back.closed);
    }

    // ── GithubPullRequest ──────────────────────────────────────────

    #[test]
    fn github_pull_request_serde() {
        let pr = GithubPullRequest {
            id: 200,
            number: 5,
            title: "Add feature".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/pull/5".into(),
            user_login: "dev".into(),
            body: Some("body".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "o/r".into(),
        };
        let json = serde_json::to_string(&pr).unwrap();
        let back: GithubPullRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.number, 5);
        assert_eq!(back.state, "open");
    }

    // ── GithubIssue ────────────────────────────────────────────────

    #[test]
    fn github_issue_serde() {
        let issue = GithubIssue {
            id: 300,
            number: 10,
            title: "Bug".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/issues/10".into(),
            user_login: "user".into(),
            body: Some("broken".into()),
            created_at: Utc::now(),
            updated_at: Some(Utc::now()),
            repo_full_name: "o/r".into(),
            labels: vec!["bug".into(), "urgent".into()],
        };
        let json = serde_json::to_string(&issue).unwrap();
        let back: GithubIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(back.labels.len(), 2);
        assert_eq!(back.labels[0], "bug");
    }

    #[test]
    fn github_issue_empty_labels_default() {
        let json = r#"{
            "id": 1, "number": 1, "title": "t", "state": "open",
            "html_url": "http://x", "user_login": "u",
            "created_at": "2025-01-01T00:00:00Z",
            "repo_full_name": "a/b"
        }"#;
        let issue: GithubIssue = serde_json::from_str(json).unwrap();
        assert!(issue.labels.is_empty());
        assert!(issue.body.is_none());
    }

    // ── RepoPrompt ─────────────────────────────────────────────────

    #[test]
    fn repo_prompt_default() {
        let rp = RepoPrompt::default();
        assert!(rp.issue_prompt.is_none());
        assert!(rp.pr_prompt.is_none());
    }

    #[test]
    fn repo_prompt_serde() {
        let rp = RepoPrompt {
            issue_prompt: Some("handle".into()),
            pr_prompt: None,
        };
        let json = serde_json::to_string(&rp).unwrap();
        assert!(!json.contains("pr_prompt"));
        let back: RepoPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rp);
    }

    // ── Clone/Debug checks ─────────────────────────────────────────

    #[test]
    fn models_are_clone_debug() {
        let c = Config::default();
        let _c2 = c.clone();
        let _ = format!("{:?}", c);

        let ds = DaemonState::default();
        let _ds2 = ds.clone();
        let _ = format!("{:?}", ds);
    }
}

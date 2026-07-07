use std::path::PathBuf;

use crate::config;
use crate::daemon::dispatcher::AgentDispatcher;
use crate::error::{GhadError, Result};
use crate::github::{client::GithubClient, issues, pulls};
use crate::models::{GithubIssue, GithubPullRequest, ObservedRepo};
use crate::store::{PromptStore, StateStore};

/// Processes new events (issues / PRs) for observed repositories.
/// Builds context strings and resolves prompts, then dispatches to agents.
pub struct EventProcessor {
    config_dir: PathBuf,
    dispatcher: AgentDispatcher,
}

impl EventProcessor {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            dispatcher: AgentDispatcher::new(),
            config_dir,
        }
    }

    /// Process a single observed repo: check for new issues/PRs and dispatch agents.
    pub async fn process_repo(
        &self,
        repo: &ObservedRepo,
        state_store: &StateStore,
    ) -> Result<()> {
        if !repo.watch_issues && !repo.watch_prs {
            tracing::debug!("Skipping {} because no event types are watched", repo.full_name);
            return Ok(());
        }

        let (owner, name) = parse_repo_full_name(&repo.full_name)?;
        let prompt_store = PromptStore::new(&self.config_dir);
        let config = config::load_config(&self.config_dir.join("config.json"))?;
        let client = GithubClient::from_config(&config)?;

        // Determine what's new
        let seen_issues = state_store.seen_issue_ids(&repo.full_name)?;
        let seen_prs = state_store.seen_pr_ids(&repo.full_name)?;

        let fetched_issues = if repo.watch_issues {
            issues::list_issues(&client, owner, name).await?
        } else {
            Vec::new()
        };
        let fetched_prs = if repo.watch_prs {
            pulls::list_pulls(&client, owner, name).await?
        } else {
            Vec::new()
        };

        let new_issues: Vec<_> = fetched_issues
            .iter()
            .filter(|issue| !seen_issues.contains(&issue.id))
            .collect();
        let new_prs: Vec<_> = fetched_prs
            .iter()
            .filter(|pr| !seen_prs.contains(&pr.id))
            .collect();

        tracing::info!(
            "Processing {}: fetched {} issues, {} PRs; seen {} issues, {} PRs; new {} issues, {} PRs",
            repo.full_name,
            fetched_issues.len(),
            fetched_prs.len(),
            seen_issues.len(),
            seen_prs.len(),
            new_issues.len(),
            new_prs.len()
        );

        let issue_prompt = repo
            .prompt
            .clone()
            .or(prompt_store.resolve_issue_prompt(&repo.full_name)?);
        let pr_prompt = repo
            .prompt
            .clone()
            .or(prompt_store.resolve_pr_prompt(&repo.full_name)?);

        let mut handled_issue_ids = Vec::new();
        for issue in new_issues {
            if self
                .handle_issue(repo, issue, issue_prompt.as_deref(), &config)
                .await
            {
                handled_issue_ids.push(issue.id);
            }
        }

        let mut handled_pr_ids = Vec::new();
        for pr in new_prs {
            if self.handle_pr(repo, pr, pr_prompt.as_deref(), &config).await {
                handled_pr_ids.push(pr.id);
            }
        }

        state_store.mark_seen(&repo.full_name, &handled_issue_ids, &handled_pr_ids)?;

        Ok(())
    }

    async fn handle_issue(
        &self,
        repo: &ObservedRepo,
        issue: &GithubIssue,
        prompt: Option<&str>,
        config: &crate::models::Config,
    ) -> bool {
        tracing::info!(
            "Detected new issue {}#{}: {}",
            repo.full_name,
            issue.number,
            issue.title
        );
        let Some(agent) = &repo.agent else {
            tracing::info!(
                "No agent configured for {}; marking issue #{} as seen",
                repo.full_name,
                issue.number
            );
            return true;
        };

        let working_dir = config
            .default_working_dir
            .as_deref()
            .unwrap_or(&self.config_dir);
        let context = self.build_issue_context(issue, prompt);
        let dispatcher = AgentDispatcher::with_paths(config.agent_paths.clone());
        match dispatcher.dispatch(agent, working_dir, &context).await {
            Ok(child) => {
                tracing::info!(
                    "Dispatched issue {}#{} to {} (pid={:?})",
                    repo.full_name,
                    issue.number,
                    agent,
                    child.id()
                );
                true
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to dispatch issue {}#{}: {err}",
                    repo.full_name,
                    issue.number
                );
                false
            }
        }
    }

    async fn handle_pr(
        &self,
        repo: &ObservedRepo,
        pr: &GithubPullRequest,
        prompt: Option<&str>,
        config: &crate::models::Config,
    ) -> bool {
        tracing::info!(
            "Detected new PR {}#{}: {}",
            repo.full_name,
            pr.number,
            pr.title
        );
        let Some(agent) = &repo.agent else {
            tracing::info!(
                "No agent configured for {}; marking PR #{} as seen",
                repo.full_name,
                pr.number
            );
            return true;
        };

        let working_dir = config
            .default_working_dir
            .as_deref()
            .unwrap_or(&self.config_dir);
        let context = self.build_pr_context(pr, prompt);
        let dispatcher = AgentDispatcher::with_paths(config.agent_paths.clone());
        match dispatcher.dispatch(agent, working_dir, &context).await {
            Ok(child) => {
                tracing::info!(
                    "Dispatched PR {}#{} to {} (pid={:?})",
                    repo.full_name,
                    pr.number,
                    agent,
                    child.id()
                );
                true
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to dispatch PR {}#{}: {err}",
                    repo.full_name,
                    pr.number
                );
                false
            }
        }
    }

    /// Build a context string for an issue to send to an agent.
    pub fn build_issue_context(
        &self,
        issue: &GithubIssue,
        prompt: Option<&str>,
    ) -> String {
        let mut ctx = format!(
            "Repository: {}\nIssue #{}: {}\nState: {}\nAuthor: {}\nURL: {}\n",
            issue.repo_full_name,
            issue.number,
            issue.title,
            issue.state,
            issue.user_login,
            issue.html_url,
        );
        if !issue.labels.is_empty() {
            ctx.push_str(&format!("Labels: {}\n", issue.labels.join(", ")));
        }
        if let Some(body) = &issue.body {
            ctx.push_str(&format!("\nBody:\n{}\n", body));
        }
        if let Some(p) = prompt {
            ctx.push_str(&format!("\nInstructions:\n{}\n", p));
        }
        ctx
    }

    /// Build a context string for a PR to send to an agent.
    pub fn build_pr_context(
        &self,
        pr: &GithubPullRequest,
        prompt: Option<&str>,
    ) -> String {
        let mut ctx = format!(
            "Repository: {}\nPR #{}: {}\nState: {}\nAuthor: {}\nURL: {}\n",
            pr.repo_full_name,
            pr.number,
            pr.title,
            pr.state,
            pr.user_login,
            pr.html_url,
        );
        if let Some(body) = &pr.body {
            ctx.push_str(&format!("\nBody:\n{}\n", body));
        }
        if let Some(p) = prompt {
            ctx.push_str(&format!("\nInstructions:\n{}\n", p));
        }
        ctx
    }

    /// Get a reference to the dispatcher.
    pub fn dispatcher(&self) -> &AgentDispatcher {
        &self.dispatcher
    }
}

fn parse_repo_full_name(full_name: &str) -> Result<(&str, &str)> {
    let (owner, repo) = full_name.split_once('/').ok_or_else(|| {
        GhadError::PollError {
            message: format!("invalid repository name '{full_name}', expected owner/repo"),
        }
    })?;
    if owner.is_empty() || repo.is_empty() {
        return Err(GhadError::PollError {
            message: format!("invalid repository name '{full_name}', expected owner/repo"),
        });
    }
    Ok((owner, repo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_processor(dir: &std::path::Path) -> EventProcessor {
        EventProcessor::new(dir.to_path_buf())
    }

    fn make_issue() -> GithubIssue {
        GithubIssue {
            id: 1,
            number: 42,
            title: "Test bug".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/issues/42".into(),
            user_login: "reporter".into(),
            body: Some("It's broken".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "o/r".into(),
            labels: vec!["bug".into(), "p1".into()],
        }
    }

    fn make_pr() -> GithubPullRequest {
        GithubPullRequest {
            id: 2,
            number: 10,
            title: "Fix bug".into(),
            state: "open".into(),
            html_url: "https://github.com/o/r/pull/10".into(),
            user_login: "dev".into(),
            body: Some("Fixes #42".into()),
            created_at: Utc::now(),
            updated_at: None,
            repo_full_name: "o/r".into(),
        }
    }

    #[test]
    fn build_issue_context_basic() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let issue = make_issue();
        let ctx = proc.build_issue_context(&issue, None);

        assert!(ctx.contains("Repository: o/r"));
        assert!(ctx.contains("Issue #42"));
        assert!(ctx.contains("Test bug"));
        assert!(ctx.contains("Author: reporter"));
        assert!(ctx.contains("Labels: bug, p1"));
        assert!(ctx.contains("It's broken"));
    }

    #[test]
    fn build_issue_context_with_prompt() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let issue = make_issue();
        let ctx = proc.build_issue_context(&issue, Some("Fix this immediately"));

        assert!(ctx.contains("Instructions:"));
        assert!(ctx.contains("Fix this immediately"));
    }

    #[test]
    fn build_issue_context_no_body() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let mut issue = make_issue();
        issue.body = None;
        let ctx = proc.build_issue_context(&issue, None);

        assert!(!ctx.contains("Body:"));
    }

    #[test]
    fn build_issue_context_no_labels() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let mut issue = make_issue();
        issue.labels = vec![];
        let ctx = proc.build_issue_context(&issue, None);

        assert!(!ctx.contains("Labels:"));
    }

    #[test]
    fn build_pr_context_basic() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let pr = make_pr();
        let ctx = proc.build_pr_context(&pr, None);

        assert!(ctx.contains("Repository: o/r"));
        assert!(ctx.contains("PR #10"));
        assert!(ctx.contains("Fix bug"));
        assert!(ctx.contains("Author: dev"));
        assert!(ctx.contains("Fixes #42"));
    }

    #[test]
    fn build_pr_context_with_prompt() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let pr = make_pr();
        let ctx = proc.build_pr_context(&pr, Some("Review carefully"));

        assert!(ctx.contains("Instructions:"));
        assert!(ctx.contains("Review carefully"));
    }

    #[test]
    fn build_pr_context_no_body() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let mut pr = make_pr();
        pr.body = None;
        let ctx = proc.build_pr_context(&pr, None);

        assert!(!ctx.contains("Body:"));
    }

    #[tokio::test]
    async fn process_repo_no_watches_no_crash() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let state_store = StateStore::new(tmp.path());
        let repo = ObservedRepo {
            full_name: "test/repo".into(),
            url: "https://github.com/test/repo".into(),
            watch_issues: false,
            watch_prs: false,
            agent: None,
            prompt: None,
            poll_interval_secs: None,
            added_at: Utc::now(),
        };
        let result = proc.process_repo(&repo, &state_store).await;
        assert!(result.is_ok());
    }

    #[test]
    fn parse_repo_full_name_rejects_invalid_names() {
        assert!(parse_repo_full_name("owner/repo").is_ok());
        assert!(parse_repo_full_name("owner").is_err());
        assert!(parse_repo_full_name("/repo").is_err());
        assert!(parse_repo_full_name("owner/").is_err());
    }

    #[test]
    fn dispatcher_accessible() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let _d = proc.dispatcher();
    }
}

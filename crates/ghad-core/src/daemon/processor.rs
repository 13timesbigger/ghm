use std::path::PathBuf;

use crate::daemon::dispatcher::AgentDispatcher;
use crate::error::Result;
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
        let prompt_store = PromptStore::new(&self.config_dir);

        // Determine what's new
        let seen_issues = state_store.seen_issue_ids(&repo.full_name)?;
        let seen_prs = state_store.seen_pr_ids(&repo.full_name)?;

        // In a real implementation, we would call the GitHub API here.
        // For now, we log and return — the actual fetching is done by the
        // caller or integrated with the GithubClient.
        tracing::debug!(
            "Processing {}: seen {} issues, {} PRs",
            repo.full_name,
            seen_issues.len(),
            seen_prs.len()
        );

        // Resolve prompts
        let _issue_prompt = prompt_store.resolve_issue_prompt(&repo.full_name)?;
        let _pr_prompt = prompt_store.resolve_pr_prompt(&repo.full_name)?;

        Ok(())
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
    async fn process_repo_no_crash() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let state_store = StateStore::new(tmp.path());
        let repo = ObservedRepo {
            full_name: "test/repo".into(),
            url: "https://github.com/test/repo".into(),
            watch_issues: true,
            watch_prs: true,
            agent: None,
            prompt: None,
            poll_interval_secs: None,
            added_at: Utc::now(),
        };
        // Should succeed without error (no actual API calls)
        let result = proc.process_repo(&repo, &state_store).await;
        assert!(result.is_ok());
    }

    #[test]
    fn dispatcher_accessible() {
        let tmp = TempDir::new().unwrap();
        let proc = make_processor(tmp.path());
        let _d = proc.dispatcher();
    }
}

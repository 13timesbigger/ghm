use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{GhmError, Result};
use crate::models::{AgentPaths, AgentType};

/// Dispatches prompts to AI coding agent CLI subprocesses.
#[derive(Debug, Clone)]
pub struct AgentDispatcher {
    agent_paths: AgentPaths,
}

impl AgentDispatcher {
    /// Create a dispatcher with default agent paths (looks up agents on `$PATH`).
    pub fn new() -> Self {
        Self {
            agent_paths: AgentPaths::default(),
        }
    }

    /// Create a dispatcher with custom agent paths.
    pub fn with_paths(paths: AgentPaths) -> Self {
        Self {
            agent_paths: paths,
        }
    }

    /// Get the binary name/path for an agent.
    pub fn agent_binary(&self, agent: &AgentType) -> PathBuf {
        match agent {
            AgentType::Codex => self
                .agent_paths
                .codex
                .clone()
                .unwrap_or_else(|| PathBuf::from("codex")),
            AgentType::Agy => self
                .agent_paths
                .agy
                .clone()
                .unwrap_or_else(|| PathBuf::from("agy")),
            AgentType::Claude => self
                .agent_paths
                .claude
                .clone()
                .unwrap_or_else(|| PathBuf::from("claude")),
            AgentType::Copilot => self
                .agent_paths
                .copilot
                .clone()
                .unwrap_or_else(|| PathBuf::from("copilot")),
        }
    }

    /// Build the command-line arguments for the given agent type.
    pub fn build_args(
        &self,
        agent: &AgentType,
        working_dir: &Path,
        prompt: &str,
    ) -> Vec<String> {
        let dir = working_dir.to_string_lossy().to_string();
        match agent {
            AgentType::Codex => vec![
                "--cd".into(),
                dir,
                "--yolo".into(),
                "--model".into(),
                "gpt-5.5".into(),
                prompt.into(),
            ],
            AgentType::Agy => vec![
                "--dangerously-skip-permissions".into(),
                "--dir".into(),
                dir,
                "--prompt".into(),
                prompt.into(),
            ],
            AgentType::Claude => vec![
                "--add-dir".into(),
                dir,
                "--dangerously-skip-permissions".into(),
                prompt.into(),
            ],
            AgentType::Copilot => vec![
                "--prompt".into(),
                prompt.into(),
                "--add-dir".into(),
                dir,
                "--autopilot".into(),
            ],
        }
    }

    /// Dispatch an agent subprocess. Returns the child process handle.
    pub async fn dispatch(
        &self,
        agent: &AgentType,
        working_dir: &Path,
        prompt: &str,
    ) -> Result<tokio::process::Child> {
        let binary = self.agent_binary(agent);
        let args = self.build_args(agent, working_dir, prompt);

        tracing::info!(
            "Dispatching {} with args: {:?}",
            agent,
            args,
        );

        let child = Command::new(&binary)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GhmError::AgentDispatch {
                agent: agent.to_string(),
                message: format!("failed to spawn {}: {e}", binary.display()),
            })?;

        Ok(child)
    }
}

impl Default for AgentDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn default_binary_names() {
        let d = AgentDispatcher::new();
        assert_eq!(d.agent_binary(&AgentType::Codex), PathBuf::from("codex"));
        assert_eq!(d.agent_binary(&AgentType::Agy), PathBuf::from("agy"));
        assert_eq!(d.agent_binary(&AgentType::Claude), PathBuf::from("claude"));
        assert_eq!(d.agent_binary(&AgentType::Copilot), PathBuf::from("copilot"));
    }

    #[test]
    fn custom_binary_paths() {
        let paths = AgentPaths {
            codex: Some(PathBuf::from("/opt/codex")),
            claude: Some(PathBuf::from("/opt/claude")),
            ..Default::default()
        };
        let d = AgentDispatcher::with_paths(paths);
        assert_eq!(d.agent_binary(&AgentType::Codex), PathBuf::from("/opt/codex"));
        assert_eq!(d.agent_binary(&AgentType::Claude), PathBuf::from("/opt/claude"));
        // Defaults for non-overridden
        assert_eq!(d.agent_binary(&AgentType::Agy), PathBuf::from("agy"));
        assert_eq!(d.agent_binary(&AgentType::Copilot), PathBuf::from("copilot"));
    }

    #[test]
    fn build_args_codex() {
        let d = AgentDispatcher::new();
        let args = d.build_args(&AgentType::Codex, Path::new("/work"), "fix bugs");
        assert_eq!(
            args,
            vec!["--cd", "/work", "--yolo", "--model", "gpt-5.5", "fix bugs"]
        );
    }

    #[test]
    fn build_args_agy() {
        let d = AgentDispatcher::new();
        let args = d.build_args(&AgentType::Agy, Path::new("/project"), "review code");
        assert_eq!(
            args,
            vec![
                "--dangerously-skip-permissions",
                "--dir",
                "/project",
                "--prompt",
                "review code"
            ]
        );
    }

    #[test]
    fn build_args_claude() {
        let d = AgentDispatcher::new();
        let args = d.build_args(&AgentType::Claude, Path::new("/src"), "analyze");
        assert_eq!(
            args,
            vec!["--add-dir", "/src", "--dangerously-skip-permissions", "analyze"]
        );
    }

    #[test]
    fn build_args_copilot() {
        let d = AgentDispatcher::new();
        let args = d.build_args(&AgentType::Copilot, Path::new("/dir"), "help");
        assert_eq!(
            args,
            vec!["--prompt", "help", "--add-dir", "/dir", "--autopilot"]
        );
    }

    #[test]
    fn dispatcher_is_clone_debug() {
        let d = AgentDispatcher::new();
        let d2 = d.clone();
        let _ = format!("{:?}", d2);
    }

    #[test]
    fn dispatcher_default_trait() {
        let d = AgentDispatcher::default();
        assert_eq!(d.agent_binary(&AgentType::Codex), PathBuf::from("codex"));
    }

    #[tokio::test]
    async fn dispatch_nonexistent_binary_fails() {
        let d = AgentDispatcher::with_paths(AgentPaths {
            codex: Some(PathBuf::from("/nonexistent/binary/codex_xxxxx")),
            ..Default::default()
        });
        let result = d
            .dispatch(&AgentType::Codex, Path::new("/tmp"), "test")
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("codex"));
    }

    #[tokio::test]
    async fn dispatch_echo_succeeds() {
        // Use 'echo' as a stand-in for a real agent
        let d = AgentDispatcher::with_paths(AgentPaths {
            codex: Some(PathBuf::from("echo")),
            ..Default::default()
        });
        let result = d
            .dispatch(&AgentType::Codex, Path::new("/tmp"), "hello")
            .await;
        assert!(result.is_ok());
        let output = result.unwrap().wait_with_output().await.unwrap();
        assert!(output.status.success());
    }
}

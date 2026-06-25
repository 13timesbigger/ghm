use std::path::{Path, PathBuf};

use crate::error::{GhmError, Result};
use crate::models::{DaemonState, ObservedRepos, Prompts};

/// File names for each store.
const OBSERVED_FILE: &str = "observed.json";
const PROMPTS_FILE: &str = "prompts.json";
const STATE_FILE: &str = "state.json";

// ── Helpers ────────────────────────────────────────────────────────

/// Atomically write `data` to `path` by writing to a temp file first then renaming.
fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GhmError::StoreWrite { source: e })?;
    }
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, data).map_err(|e| GhmError::StoreWrite { source: e })?;
    std::fs::rename(&tmp_path, path).map_err(|e| GhmError::AtomicRename {
        from: tmp_path,
        to: path.to_path_buf(),
        source: e,
    })?;
    Ok(())
}

fn load_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let data = std::fs::read_to_string(path).map_err(|e| GhmError::StoreRead { source: e })?;
    let value: T =
        serde_json::from_str(&data).map_err(|e| GhmError::StoreParse { source: e })?;
    Ok(value)
}

fn save_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(|e| GhmError::StoreParse { source: e })?;
    atomic_write(path, json.as_bytes())
}

// ═══════════════════════════════════════════════════════════════════
// ObservedStore
// ═══════════════════════════════════════════════════════════════════

/// Manages `observed.json` – the list of monitored repositories.
#[derive(Debug, Clone)]
pub struct ObservedStore {
    path: PathBuf,
}

impl ObservedStore {
    /// Create a store backed by the given directory (file = `<dir>/observed.json`).
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(OBSERVED_FILE),
        }
    }

    /// Return the path to the backing file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load the observed repos, returning empty defaults if the file doesn't exist.
    pub fn load(&self) -> Result<ObservedRepos> {
        load_json(&self.path)
    }

    /// Save observed repos atomically.
    pub fn save(&self, repos: &ObservedRepos) -> Result<()> {
        save_json(&self.path, repos)
    }

    /// Add a repo if it doesn't already exist (by full_name). Returns true if added.
    pub fn add_repo(&self, repo: crate::models::ObservedRepo) -> Result<bool> {
        let mut repos = self.load()?;
        if repos
            .repositories
            .iter()
            .any(|r| r.full_name == repo.full_name)
        {
            return Ok(false);
        }
        repos.repositories.push(repo);
        self.save(&repos)?;
        Ok(true)
    }

    /// Remove a repo by full_name. Returns true if it was found and removed.
    pub fn remove_repo(&self, full_name: &str) -> Result<bool> {
        let mut repos = self.load()?;
        let before = repos.repositories.len();
        repos.repositories.retain(|r| r.full_name != full_name);
        if repos.repositories.len() == before {
            return Ok(false);
        }
        self.save(&repos)?;
        Ok(true)
    }
}

// ═══════════════════════════════════════════════════════════════════
// PromptStore
// ═══════════════════════════════════════════════════════════════════

/// Manages `prompts.json` – global and per-repo prompt templates.
#[derive(Debug, Clone)]
pub struct PromptStore {
    path: PathBuf,
}

impl PromptStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(PROMPTS_FILE),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Prompts> {
        load_json(&self.path)
    }

    pub fn save(&self, prompts: &Prompts) -> Result<()> {
        save_json(&self.path, prompts)
    }

    /// Resolve the effective issue prompt for a repo (repo-specific overrides global).
    pub fn resolve_issue_prompt(&self, repo_full_name: &str) -> Result<Option<String>> {
        let prompts = self.load()?;
        if let Some(rp) = prompts.repos.get(repo_full_name) {
            if rp.issue_prompt.is_some() {
                return Ok(rp.issue_prompt.clone());
            }
        }
        Ok(prompts.global.issue_prompt.clone())
    }

    /// Resolve the effective PR prompt for a repo (repo-specific overrides global).
    pub fn resolve_pr_prompt(&self, repo_full_name: &str) -> Result<Option<String>> {
        let prompts = self.load()?;
        if let Some(rp) = prompts.repos.get(repo_full_name) {
            if rp.pr_prompt.is_some() {
                return Ok(rp.pr_prompt.clone());
            }
        }
        Ok(prompts.global.pr_prompt.clone())
    }
}

// ═══════════════════════════════════════════════════════════════════
// StateStore
// ═══════════════════════════════════════════════════════════════════

/// Manages `state.json` – daemon PID, status, and per-repo seen events.
#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(STATE_FILE),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<DaemonState> {
        load_json(&self.path)
    }

    pub fn save(&self, state: &DaemonState) -> Result<()> {
        save_json(&self.path, state)
    }

    /// Mark the daemon as running with the given PID.
    pub fn set_running(&self, pid: u32) -> Result<()> {
        let mut state = self.load()?;
        state.daemon_pid = Some(pid);
        state.daemon_status = Some("running".into());
        self.save(&state)
    }

    /// Mark the daemon as stopped.
    pub fn set_stopped(&self) -> Result<()> {
        let mut state = self.load()?;
        state.daemon_pid = None;
        state.daemon_status = Some("stopped".into());
        self.save(&state)
    }

    /// Record that we have seen a set of issue/PR IDs for a repo.
    pub fn mark_seen(
        &self,
        repo_full_name: &str,
        issue_ids: &[u64],
        pr_ids: &[u64],
    ) -> Result<()> {
        let mut state = self.load()?;
        let entry = state
            .repos
            .entry(repo_full_name.to_string())
            .or_default();
        for id in issue_ids {
            if !entry.seen.issue_ids.contains(id) {
                entry.seen.issue_ids.push(*id);
            }
        }
        for id in pr_ids {
            if !entry.seen.pr_ids.contains(id) {
                entry.seen.pr_ids.push(*id);
            }
        }
        entry.last_poll = Some(chrono::Utc::now());
        self.save(&state)
    }

    /// Get seen issue IDs for a repo.
    pub fn seen_issue_ids(&self, repo_full_name: &str) -> Result<Vec<u64>> {
        let state = self.load()?;
        Ok(state
            .repos
            .get(repo_full_name)
            .map(|r| r.seen.issue_ids.clone())
            .unwrap_or_default())
    }

    /// Get seen PR IDs for a repo.
    pub fn seen_pr_ids(&self, repo_full_name: &str) -> Result<Vec<u64>> {
        let state = self.load()?;
        Ok(state
            .repos
            .get(repo_full_name)
            .map(|r| r.seen.pr_ids.clone())
            .unwrap_or_default())
    }
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // ── atomic_write ───────────────────────────────────────────────

    #[test]
    fn atomic_write_creates_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.json");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("a").join("b").join("file.json");
        atomic_write(&path, b"data").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn atomic_write_overwrites() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("over.json");
        atomic_write(&path, b"first").unwrap();
        atomic_write(&path, b"second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    // ── ObservedStore ──────────────────────────────────────────────

    #[test]
    fn observed_store_path() {
        let tmp = TempDir::new().unwrap();
        let store = ObservedStore::new(tmp.path());
        assert!(store.path().ends_with("observed.json"));
    }

    #[test]
    fn observed_store_load_empty() {
        let tmp = TempDir::new().unwrap();
        let store = ObservedStore::new(tmp.path());
        let repos = store.load().unwrap();
        assert!(repos.repositories.is_empty());
    }

    #[test]
    fn observed_store_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = ObservedStore::new(tmp.path());
        let repos = ObservedRepos {
            repositories: vec![ObservedRepo {
                full_name: "a/b".into(),
                url: "https://github.com/a/b".into(),
                watch_issues: true,
                watch_prs: true,
                agent: None,
                prompt: None,
                poll_interval_secs: None,
                added_at: Utc::now(),
            }],
        };
        store.save(&repos).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.repositories.len(), 1);
        assert_eq!(loaded.repositories[0].full_name, "a/b");
    }

    #[test]
    fn observed_store_add_repo() {
        let tmp = TempDir::new().unwrap();
        let store = ObservedStore::new(tmp.path());
        let repo = ObservedRepo {
            full_name: "x/y".into(),
            url: "https://github.com/x/y".into(),
            watch_issues: true,
            watch_prs: true,
            agent: Some(AgentType::Codex),
            prompt: None,
            poll_interval_secs: None,
            added_at: Utc::now(),
        };
        assert!(store.add_repo(repo.clone()).unwrap());
        // Duplicate should return false
        assert!(!store.add_repo(repo).unwrap());
        let repos = store.load().unwrap();
        assert_eq!(repos.repositories.len(), 1);
    }

    #[test]
    fn observed_store_remove_repo() {
        let tmp = TempDir::new().unwrap();
        let store = ObservedStore::new(tmp.path());
        let repo = ObservedRepo {
            full_name: "x/y".into(),
            url: "https://github.com/x/y".into(),
            watch_issues: true,
            watch_prs: false,
            agent: None,
            prompt: None,
            poll_interval_secs: None,
            added_at: Utc::now(),
        };
        store.add_repo(repo).unwrap();
        assert!(store.remove_repo("x/y").unwrap());
        assert!(!store.remove_repo("x/y").unwrap()); // already removed
        let repos = store.load().unwrap();
        assert!(repos.repositories.is_empty());
    }

    #[test]
    fn observed_store_invalid_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(OBSERVED_FILE);
        std::fs::write(&path, "INVALID").unwrap();
        let store = ObservedStore::new(tmp.path());
        assert!(store.load().is_err());
    }

    // ── PromptStore ────────────────────────────────────────────────

    #[test]
    fn prompt_store_path() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        assert!(store.path().ends_with("prompts.json"));
    }

    #[test]
    fn prompt_store_load_empty() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        let prompts = store.load().unwrap();
        assert!(prompts.repos.is_empty());
        assert!(prompts.global.issue_prompt.is_none());
    }

    #[test]
    fn prompt_store_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        let mut repos = HashMap::new();
        repos.insert(
            "a/b".to_string(),
            RepoPrompt {
                issue_prompt: Some("repo issue".into()),
                pr_prompt: None,
            },
        );
        let prompts = Prompts {
            global: PromptConfig {
                issue_prompt: Some("global issue".into()),
                pr_prompt: Some("global pr".into()),
            },
            repos,
        };
        store.save(&prompts).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.global.issue_prompt, Some("global issue".into()));
        assert!(loaded.repos.contains_key("a/b"));
    }

    #[test]
    fn prompt_store_resolve_issue_repo_override() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        let mut repos = HashMap::new();
        repos.insert(
            "a/b".to_string(),
            RepoPrompt {
                issue_prompt: Some("repo-level".into()),
                pr_prompt: None,
            },
        );
        let prompts = Prompts {
            global: PromptConfig {
                issue_prompt: Some("global-level".into()),
                pr_prompt: None,
            },
            repos,
        };
        store.save(&prompts).unwrap();
        let resolved = store.resolve_issue_prompt("a/b").unwrap();
        assert_eq!(resolved, Some("repo-level".into()));
    }

    #[test]
    fn prompt_store_resolve_issue_falls_back_to_global() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        let prompts = Prompts {
            global: PromptConfig {
                issue_prompt: Some("global".into()),
                pr_prompt: None,
            },
            repos: HashMap::new(),
        };
        store.save(&prompts).unwrap();
        let resolved = store.resolve_issue_prompt("unknown/repo").unwrap();
        assert_eq!(resolved, Some("global".into()));
    }

    #[test]
    fn prompt_store_resolve_pr_prompt() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        let mut repos = HashMap::new();
        repos.insert(
            "a/b".to_string(),
            RepoPrompt {
                issue_prompt: None,
                pr_prompt: Some("repo pr".into()),
            },
        );
        let prompts = Prompts {
            global: PromptConfig {
                issue_prompt: None,
                pr_prompt: Some("global pr".into()),
            },
            repos,
        };
        store.save(&prompts).unwrap();
        assert_eq!(
            store.resolve_pr_prompt("a/b").unwrap(),
            Some("repo pr".into())
        );
        assert_eq!(
            store.resolve_pr_prompt("other").unwrap(),
            Some("global pr".into())
        );
    }

    #[test]
    fn prompt_store_resolve_returns_none() {
        let tmp = TempDir::new().unwrap();
        let store = PromptStore::new(tmp.path());
        // No file at all → defaults → no prompts
        assert!(store.resolve_issue_prompt("a/b").unwrap().is_none());
        assert!(store.resolve_pr_prompt("a/b").unwrap().is_none());
    }

    // ── StateStore ─────────────────────────────────────────────────

    #[test]
    fn state_store_path() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        assert!(store.path().ends_with("state.json"));
    }

    #[test]
    fn state_store_load_empty() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        let state = store.load().unwrap();
        assert!(state.daemon_pid.is_none());
    }

    #[test]
    fn state_store_set_running_and_stopped() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        store.set_running(1234).unwrap();
        let state = store.load().unwrap();
        assert_eq!(state.daemon_pid, Some(1234));
        assert_eq!(state.daemon_status, Some("running".into()));

        store.set_stopped().unwrap();
        let state = store.load().unwrap();
        assert!(state.daemon_pid.is_none());
        assert_eq!(state.daemon_status, Some("stopped".into()));
    }

    #[test]
    fn state_store_mark_seen() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        store.mark_seen("a/b", &[1, 2, 3], &[10, 20]).unwrap();
        let issues = store.seen_issue_ids("a/b").unwrap();
        assert_eq!(issues, vec![1, 2, 3]);
        let prs = store.seen_pr_ids("a/b").unwrap();
        assert_eq!(prs, vec![10, 20]);
    }

    #[test]
    fn state_store_mark_seen_no_duplicates() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        store.mark_seen("a/b", &[1, 2], &[]).unwrap();
        store.mark_seen("a/b", &[2, 3], &[10]).unwrap();
        let issues = store.seen_issue_ids("a/b").unwrap();
        assert_eq!(issues, vec![1, 2, 3]);
    }

    #[test]
    fn state_store_seen_ids_unknown_repo() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        assert!(store.seen_issue_ids("nope").unwrap().is_empty());
        assert!(store.seen_pr_ids("nope").unwrap().is_empty());
    }

    #[test]
    fn state_store_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let store = StateStore::new(tmp.path());
        let state = DaemonState {
            daemon_pid: Some(42),
            daemon_status: Some("running".into()),
            last_poll: Some(Utc::now()),
            repos: HashMap::new(),
        };
        store.save(&state).unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.daemon_pid, Some(42));
    }

    #[test]
    fn state_store_invalid_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(STATE_FILE);
        std::fs::write(&path, "BROKEN").unwrap();
        let store = StateStore::new(tmp.path());
        assert!(store.load().is_err());
    }

    // ── clone / debug ──────────────────────────────────────────────

    #[test]
    fn stores_are_clone_debug() {
        let tmp = TempDir::new().unwrap();
        let os = ObservedStore::new(tmp.path());
        let _os2 = os.clone();
        let _ = format!("{:?}", os);

        let ps = PromptStore::new(tmp.path());
        let _ps2 = ps.clone();
        let _ = format!("{:?}", ps);

        let ss = StateStore::new(tmp.path());
        let _ss2 = ss.clone();
        let _ = format!("{:?}", ss);
    }
}

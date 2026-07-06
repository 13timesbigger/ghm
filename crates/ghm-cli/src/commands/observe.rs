use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::models::ObservedRepo;
use ghm_core::store::ObservedStore;

use crate::cli::ObserveArgs;
use crate::output::{self, ObservedRow};

/// Parse a repository identifier from either "owner/repo" format or a GitHub URL.
fn parse_repo_identifier(input: &str) -> Result<String> {
    // Handle GitHub URLs
    if input.starts_with("https://github.com/") || input.starts_with("http://github.com/") {
        let path = input
            .trim_start_matches("https://github.com/")
            .trim_start_matches("http://github.com/")
            .trim_end_matches('/')
            .trim_end_matches(".git");
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Ok(format!("{}/{}", parts[0], parts[1]));
        }
        anyhow::bail!("Invalid GitHub URL: '{}'", input);
    }

    // Handle "owner/repo" format
    let parts: Vec<&str> = input.splitn(2, '/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        return Ok(input.to_string());
    }

    anyhow::bail!(
        "Invalid repository format: '{}'. Expected 'owner/repo' or a GitHub URL.",
        input
    );
}

pub async fn handle_observe(args: &ObserveArgs) -> Result<()> {
    let repo_input = args
        .repo
        .as_ref()
        .context("Repository name is required")?;

    let repo_name = parse_repo_identifier(repo_input)?;

    let _config = load_default_config().context(
        "Failed to load configuration. Run 'ghad auth configure' first.",
    )?;

    let watch_issues = args.issues;
    let watch_prs = args.prs;
    let agent = args.agent().map(|s| match s {
        "codex" => ghm_core::models::AgentType::Codex,
        "agy" => ghm_core::models::AgentType::Agy,
        "claude" => ghm_core::models::AgentType::Claude,
        "copilot" => ghm_core::models::AgentType::Copilot,
        _ => unreachable!(),
    });
    let prompt = args.prompt.clone();

    let (watch_issues, watch_prs) = if !watch_issues && !watch_prs {
        (true, true)
    } else {
        (watch_issues, watch_prs)
    };

    let sp = output::spinner(&format!("Adding '{}' to observed repositories...", repo_name));

    let config_dir = ghm_core::config::default_config_dir()?;
    let store = ObservedStore::new(&config_dir);
    let observed = ObservedRepo {
        full_name: repo_name.clone(),
        url: format!("https://github.com/{}", repo_name),
        watch_issues,
        watch_prs,
        agent,
        prompt: prompt.clone(),
        poll_interval_secs: None,
        added_at: chrono::Utc::now(),
    };
    
    if !store.add_repo(observed).context("Failed to save observed repos")? {
        sp.finish_and_clear();
        output::print_warning(&format!("Repository '{}' is already being observed", repo_name));
        return Ok(());
    }

    sp.finish_and_clear();

    output::print_success(&format!("Now observing '{}'", repo_name));
    if watch_issues {
        output::print_info("  • Watching issues");
    }
    if watch_prs {
        output::print_info("  • Watching pull requests");
    }
    if let Some(agent_name) = args.agent() {
        output::print_info(&format!("  • AI agent: {:?}", agent_name));
    }
    if let Some(ref p) = prompt {
        output::print_info(&format!("  • Prompt: {}", p));
    }

    Ok(())
}

pub async fn handle_list() -> Result<()> {
    let config_dir = ghm_core::config::default_config_dir()?;
    let store = ObservedStore::new(&config_dir);
    let repos = store.load().unwrap_or_default();

    let rows: Vec<ObservedRow> = repos.repositories
        .iter()
        .map(|obs| ObservedRow {
            repo: obs.full_name.clone(),
            watch_issues: obs.watch_issues,
            watch_prs: obs.watch_prs,
            agent: obs.agent.as_ref().map(|a| format!("{:?}", a)).unwrap_or_else(|| "—".to_string()),
            prompt: obs.prompt.clone().unwrap_or_default(),
        })
        .collect();

    output::print_observed_table(&rows);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_owner_slash_repo() {
        let result = parse_repo_identifier("myorg/myrepo").unwrap();
        assert_eq!(result, "myorg/myrepo");
    }

    #[test]
    fn test_parse_repo_https_url() {
        let result =
            parse_repo_identifier("https://github.com/myorg/myrepo").unwrap();
        assert_eq!(result, "myorg/myrepo");
    }

    #[test]
    fn test_parse_repo_https_url_trailing_slash() {
        let result =
            parse_repo_identifier("https://github.com/myorg/myrepo/").unwrap();
        assert_eq!(result, "myorg/myrepo");
    }

    #[test]
    fn test_parse_repo_https_url_git_suffix() {
        let result =
            parse_repo_identifier("https://github.com/myorg/myrepo.git").unwrap();
        assert_eq!(result, "myorg/myrepo");
    }

    #[test]
    fn test_parse_repo_http_url() {
        let result =
            parse_repo_identifier("http://github.com/myorg/myrepo").unwrap();
        assert_eq!(result, "myorg/myrepo");
    }

    #[test]
    fn test_parse_repo_invalid_no_slash() {
        let result = parse_repo_identifier("justaname");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_invalid_empty_owner() {
        let result = parse_repo_identifier("/myrepo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_invalid_empty_repo() {
        let result = parse_repo_identifier("myorg/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_invalid_url_no_repo() {
        let result = parse_repo_identifier("https://github.com/myorg");
        assert!(result.is_err());
    }
}

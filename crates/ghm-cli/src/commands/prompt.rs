use anyhow::{Context, Result};

use ghm_core::store::PromptStore;

use crate::cli::{PromptArgs, PromptScope};
use crate::output;

/// Handle the `ghad prompt` command.
///
/// Sets a global or repository-specific prompt for AI agents.
pub async fn handle_prompt(args: &PromptArgs) -> Result<()> {
    let config_dir = ghm_core::config::default_config_dir()?;
    let store = PromptStore::new(&config_dir);
    let mut prompts = store.load().unwrap_or_default();

    let repo_name = args.repo.clone().unwrap_or_else(|| "*".to_string());

    if args.global {
        match args.scope() {
            PromptScope::All => {
                prompts.global.issue_prompt = Some(args.prompt.clone());
                prompts.global.pr_prompt = Some(args.prompt.clone());
            }
            PromptScope::Issue => prompts.global.issue_prompt = Some(args.prompt.clone()),
            PromptScope::Pr => prompts.global.pr_prompt = Some(args.prompt.clone()),
        }
    } else {
        let repo_prompt = prompts.repos.entry(repo_name.clone()).or_default();
        match args.scope() {
            PromptScope::All => {
                repo_prompt.issue_prompt = Some(args.prompt.clone());
                repo_prompt.pr_prompt = Some(args.prompt.clone());
            }
            PromptScope::Issue => repo_prompt.issue_prompt = Some(args.prompt.clone()),
            PromptScope::Pr => repo_prompt.pr_prompt = Some(args.prompt.clone()),
        }
    }

    store.save(&prompts).context("Failed to save prompt configuration")?;

    let scope_desc = if args.global {
        "global".to_string()
    } else {
        format!("repository '{}'", repo_name)
    };

    let event_desc = match args.scope() {
        PromptScope::All => "all events",
        PromptScope::Issue => "issues only",
        PromptScope::Pr => "pull requests only",
    };

    output::print_success(&format!(
        "Prompt set for {} ({})",
        scope_desc, event_desc
    ));
    output::print_info(&format!("Prompt: {}", args.prompt));

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

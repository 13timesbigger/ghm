use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::github::projects;

use crate::output::{self, ProjectRow};

/// Handle the `ghm project list` command.
///
/// Lists GitHub Projects (v2), optionally filtered by organization.
pub async fn handle_list(org: Option<String>) -> Result<()> {
    let config = load_default_config().context(
        "Failed to load configuration. Run 'ghm auth configure' first.",
    )?;

    let label = match &org {
        Some(name) => format!("Fetching projects for '{}'...", name),
        None => "Fetching your projects...".to_string(),
    };
    let sp = output::spinner(&label);

    let client = ghm_core::github::client::GithubClient::from_config(&config)?;
    let projects = match &org {
        Some(name) => projects::list_projects_by_org(&client, name).await
            .context(format!("Failed to fetch projects for org '{}'", name))?,
        None => projects::list_projects(&client).await
            .context("Failed to fetch user projects")?,
    };

    sp.finish_and_clear();

    let rows: Vec<ProjectRow> = projects
        .into_iter()
        .map(|proj| ProjectRow {
            title: proj.title,
            description: proj.short_description.unwrap_or_default(),
            status: if proj.closed { "closed".to_string() } else { "open".to_string() },
            item_count: 0,
        })
        .collect();

    output::print_projects_table(&rows);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}

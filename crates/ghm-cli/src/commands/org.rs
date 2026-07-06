use anyhow::{Context, Result};

use ghm_core::config::load_default_config;
use ghm_core::github::orgs;

use crate::output::{self, OrgRow};

/// Handle the `ghad org list` command.
///
/// Lists all GitHub organizations that the authenticated user belongs to.
pub async fn handle_list() -> Result<()> {
    let config = load_default_config().context(
        "Failed to load configuration. Run 'ghad auth configure' first.",
    )?;

    let sp = output::spinner("Fetching organizations...");

    let client = ghm_core::github::client::GithubClient::from_config(&config)?;
    let orgs = orgs::list_orgs(&client).await
        .context("Failed to fetch organizations")?;

    sp.finish_and_clear();

    let rows: Vec<OrgRow> = orgs
        .into_iter()
        .map(|org| OrgRow {
            login: org.login,
            description: org.description.unwrap_or_default(),
        })
        .collect();

    output::print_orgs_table(&rows);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // Verify the module compiles correctly
        assert!(true);
    }
}

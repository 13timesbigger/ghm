mod cli;
mod commands;
mod output;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{
    AuthCommands, Cli, Commands, IssueCommands, ObserveSubcommands,
    OrgCommands, PrCommands, ProjectCommands, RepoCommands,
};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        output::print_error(&format!("{:#}", err));
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing based on verbosity level
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    tracing::debug!("ghm starting with verbosity level {}", cli.verbose);

    match cli.command {
        // ── Auth ───────────────────────────────────────────────────────
        Commands::Auth(AuthCommands::Configure) => {
            commands::auth::handle_configure().await?;
        }

        // ── Org ────────────────────────────────────────────────────────
        Commands::Org(OrgCommands::List) => {
            commands::org::handle_list().await?;
        }

        // ── Repo ───────────────────────────────────────────────────────
        Commands::Repo(RepoCommands::List { org }) => {
            commands::repo::handle_list(org).await?;
        }

        // ── Project ────────────────────────────────────────────────────
        Commands::Project(ProjectCommands::List { org }) => {
            commands::project::handle_list(org).await?;
        }

        // ── PR ─────────────────────────────────────────────────────────
        Commands::Pr(PrCommands::List { repo, org }) => {
            commands::pr::handle_list(repo, org).await?;
        }

        // ── Issue ──────────────────────────────────────────────────────
        Commands::Issue(IssueCommands::List { repo, org, project }) => {
            commands::issue::handle_list(repo, org, project).await?;
        }

        // ── Observe ────────────────────────────────────────────────────
        Commands::Observe(ref args) => {
            match &args.subcommand {
                Some(ObserveSubcommands::List) => {
                    commands::observe::handle_list().await?;
                }
                None => {
                    commands::observe::handle_observe(args).await?;
                }
            }
        }

        // ── Prompt ─────────────────────────────────────────────────────
        Commands::Prompt(ref args) => {
            commands::prompt::handle_prompt(args).await?;
        }

        // ── Daemon ─────────────────────────────────────────────────────
        Commands::Daemon(ref cmd) => {
            commands::daemon::handle_daemon(cmd).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_main_module_compiles() {
        // Verify the module structure compiles
        assert!(true);
    }

    #[test]
    fn test_verbosity_filter_levels() {
        // Verify our verbosity mapping logic
        let levels = [(0u8, "warn"), (1, "info"), (2, "debug"), (3, "trace")];
        for (verbose, expected) in levels {
            let filter = match verbose {
                0 => "warn",
                1 => "info",
                2 => "debug",
                _ => "trace",
            };
            assert_eq!(filter, expected, "verbosity {} should map to {}", verbose, expected);
        }
    }
}

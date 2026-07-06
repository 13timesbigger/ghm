use clap::{Parser, Subcommand};

/// Github Activity to Agents Dispatcher (GHAAD) — A CLI tool for tracking
/// GitHub activity and dispatching it to agents.
///
/// Use `ghad <command> --help` for detailed information about each command.
#[derive(Parser, Debug)]
#[command(
    name = "ghad",
    version,
    about = "Github Activity to Agents Dispatcher (GHAAD)",
    long_about = "Github Activity to Agents Dispatcher (GHAAD) is a CLI tool and daemon \
        for monitoring GitHub repositories, tracking issues, pull requests, and projects \
        across your organizations, and dispatching activity to agents.\n\n\
        Get started by configuring authentication:\n  \
        ghad auth configure\n\n\
        Then list your organizations:\n  \
        ghad org list\n\n\
        And start observing repositories:\n  \
        ghad observe <repo-name>",
    propagate_version = true
)]
pub struct Cli {
    /// Enable verbose logging output (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output format (text or json)
    #[arg(long, default_value = "text", global = true)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format for command results
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text with tables and colors
    Text,
    /// Machine-readable JSON output
    Json,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage authentication credentials for the GitHub API.
    ///
    /// Configure a GitHub App installation.
    /// Your credentials are stored in the local configuration directory.
    #[command(subcommand)]
    Auth(AuthCommands),

    /// List and manage GitHub organizations.
    ///
    /// View all organizations that the authenticated user belongs to,
    /// including organization details like member counts and descriptions.
    #[command(subcommand)]
    Org(OrgCommands),

    /// List and manage GitHub repositories.
    ///
    /// View repositories across your account or filter by organization.
    /// Shows repository details including stars, language, and last updated time.
    #[command(subcommand)]
    Repo(RepoCommands),

    /// List and manage GitHub Projects (v2).
    ///
    /// View projects across your account or filter by organization.
    /// Shows project details including status and item counts.
    #[command(subcommand)]
    Project(ProjectCommands),

    /// List and manage pull requests.
    ///
    /// View pull requests across repositories with filtering options.
    /// Shows PR details including status, author, and review state.
    #[command(subcommand)]
    Pr(PrCommands),

    /// List and manage issues.
    ///
    /// View issues across repositories with filtering options by org,
    /// repo, and project. Shows issue details including status, labels,
    /// and assignees.
    #[command(subcommand)]
    Issue(IssueCommands),

    /// Observe repositories for changes.
    ///
    /// Add a repository to the watch list to monitor for new issues,
    /// pull requests, and other events. Configure AI-powered agents
    /// to automatically respond to changes.
    ///
    /// Examples:
    ///   ghad observe myorg/myrepo --issues --prs
    ///   ghad observe myorg/myrepo --prompt "Review for security issues" --claude
    ///   ghad observe list
    Observe(ObserveArgs),

    /// Manage prompts for AI-powered agents.
    ///
    /// Set global or repository-specific prompts that are used by AI agents
    /// when processing issues and pull requests. Prompts can be scoped to
    /// specific event types (issues or PRs).
    ///
    /// Examples:
    ///   ghad prompt --global "Always check for security vulnerabilities"
    ///   ghad prompt --repo myorg/myrepo --issue "Label and triage this issue"
    ///   ghad prompt --repo myorg/myrepo --pr "Review code quality"
    Prompt(PromptArgs),

    /// Manage the ghad background daemon.
    ///
    /// The daemon runs in the background and periodically checks observed
    /// repositories for changes, triggering configured AI agents when
    /// new events are detected.
    ///
    /// Examples:
    ///   ghad daemon start
    ///   ghad daemon status
    ///   ghad daemon install   # Install as a system service
    #[command(subcommand)]
    Daemon(DaemonCommands),
}

// ── Auth ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Configure GitHub authentication credentials.
    ///
    /// Interactively configure a GitHub App installation. Credentials are stored
    /// in the ghad config file.
    Configure,
}

// ── Org ────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum OrgCommands {
    /// List all GitHub organizations for the authenticated user.
    ///
    /// Displays a table of organizations including name, description,
    /// and member count. Requires authentication to be configured first.
    List,
}

// ── Repo ───────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    /// List repositories, optionally filtered by organization.
    ///
    /// Displays a table of repositories with details including name,
    /// description, primary language, star count, and last updated time.
    ///
    /// If --org and --all-orgs are not specified, lists repositories for the
    /// authenticated user.
    List {
        /// Filter repositories by organization name.
        ///
        /// Example: --org mycompany
        #[arg(long, short, conflicts_with = "all_orgs")]
        org: Option<String>,

        /// List repositories from every organization the authenticated user belongs to.
        #[arg(long, conflicts_with = "org")]
        all_orgs: bool,
    },
}

// ── Project ────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// List GitHub Projects (v2), optionally filtered by organization.
    ///
    /// Displays a table of projects with details including title,
    /// description, and status.
    ///
    /// If --org is not specified, lists projects for the authenticated user.
    List {
        /// Filter projects by organization name.
        ///
        /// Example: --org mycompany
        #[arg(long, short)]
        org: Option<String>,
    },
}

// ── PR ─────────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum PrCommands {
    /// List pull requests, optionally filtered by repository and/or organization.
    ///
    /// Displays a table of pull requests with details including title,
    /// status (open/closed/merged), author, and creation date.
    ///
    /// If neither --repo nor --org is specified, lists PRs for all observed repos.
    List {
        /// Filter by repository name (format: "owner/repo" or just "repo" if --org is set).
        ///
        /// Example: --repo myorg/myrepo
        #[arg(long, short)]
        repo: Option<String>,

        /// Filter by organization name. Combined with --repo to form "org/repo".
        ///
        /// Example: --org mycompany
        #[arg(long, short)]
        org: Option<String>,
    },
}

// ── Issue ──────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum IssueCommands {
    /// List issues, optionally filtered by repository, organization, and/or project.
    ///
    /// Displays a table of issues with details including title, status,
    /// labels, assignees, and creation date.
    ///
    /// If no filters are specified, lists issues for all observed repos.
    List {
        /// Filter by repository name (format: "owner/repo" or just "repo" if --org is set).
        ///
        /// Example: --repo myorg/myrepo
        #[arg(long, short)]
        repo: Option<String>,

        /// Filter by organization name. Combined with --repo to form "org/repo".
        ///
        /// Example: --org mycompany
        #[arg(long, short)]
        org: Option<String>,

        /// Filter by GitHub Project name.
        ///
        /// Example: --project "Sprint Board"
        #[arg(long, short)]
        project: Option<String>,
    },
}

// ── Observe ────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug)]
#[command(
    about = "Observe repositories for changes or list observed repos",
    long_about = "Add a repository to the watch list to monitor for new issues, \
        pull requests, and other events. Configure AI-powered agents \
        to automatically respond to changes.\n\n\
        Use 'ghad observe list' to view currently observed repositories.\n\n\
        Examples:\n  \
        ghad observe myorg/myrepo --issues --prs\n  \
        ghad observe myorg/myrepo --prompt \"Review for security\" --claude\n  \
        ghad observe list"
)]
pub struct ObserveArgs {
    #[command(subcommand)]
    pub subcommand: Option<ObserveSubcommands>,

    /// Repository name or URL to observe (format: "owner/repo" or full GitHub URL).
    ///
    /// Example: myorg/myrepo or https://github.com/myorg/myrepo
    pub repo: Option<String>,

    /// Monitor new issues in this repository.
    #[arg(long, default_value_t = false)]
    pub issues: bool,

    /// Monitor new pull requests in this repository.
    #[arg(long, default_value_t = false)]
    pub prs: bool,

    /// Custom prompt for the AI agent when processing events.
    ///
    /// Example: --prompt "Review this for security vulnerabilities"
    #[arg(long, short)]
    pub prompt: Option<String>,

    /// Use OpenAI Codex as the AI agent.
    #[arg(long, group = "agent")]
    pub codex: bool,

    /// Use Agy as the AI agent.
    #[arg(long, group = "agent")]
    pub agy: bool,

    /// Use Anthropic Claude as the AI agent.
    #[arg(long, group = "agent")]
    pub claude: bool,

    /// Use GitHub Copilot as the AI agent.
    #[arg(long, group = "agent")]
    pub copilot: bool,
}

#[derive(Subcommand, Debug)]
pub enum ObserveSubcommands {
    /// List all currently observed repositories.
    ///
    /// Displays a table of repositories being monitored, including
    /// what event types are being watched and any configured AI agents.
    List,
}

impl ObserveArgs {
    /// Returns the selected AI agent name, if any.
    pub fn agent(&self) -> Option<&'static str> {
        if self.codex {
            Some("codex")
        } else if self.agy {
            Some("agy")
        } else if self.claude {
            Some("claude")
        } else if self.copilot {
            Some("copilot")
        } else {
            None
        }
    }
}

// ── Prompt ─────────────────────────────────────────────────────────────

#[derive(clap::Args, Debug)]
#[command(
    about = "Manage prompts for AI-powered agents",
    long_about = "Set global or repository-specific prompts that are used by AI agents \
        when processing issues and pull requests.\n\n\
        Examples:\n  \
        ghad prompt --global \"Always check for security vulnerabilities\"\n  \
        ghad prompt --repo myorg/myrepo --issue \"Triage this issue\"\n  \
        ghad prompt --repo myorg/myrepo --pr \"Review code quality\""
)]
pub struct PromptArgs {
    /// The prompt text to set for the AI agent.
    pub prompt: String,

    /// Set this as a global prompt (applies to all repositories).
    #[arg(long, short, default_value_t = false)]
    pub global: bool,

    /// Target repository for this prompt (format: "owner/repo").
    ///
    /// Example: --repo myorg/myrepo
    #[arg(long, short)]
    pub repo: Option<String>,

    /// Apply this prompt only to issues.
    #[arg(long, group = "scope")]
    pub issue: bool,

    /// Apply this prompt only to pull requests.
    #[arg(long, group = "scope")]
    pub pr: bool,
}

impl PromptArgs {
    /// Returns the scope of this prompt.
    pub fn scope(&self) -> PromptScope {
        if self.issue {
            PromptScope::Issue
        } else if self.pr {
            PromptScope::Pr
        } else {
            PromptScope::All
        }
    }
}

/// The scope a prompt applies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptScope {
    /// Prompt applies to all event types
    All,
    /// Prompt applies only to issues
    Issue,
    /// Prompt applies only to pull requests
    Pr,
}

// ── Daemon ─────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum DaemonCommands {
    /// Start the ghad daemon in the background.
    ///
    /// The daemon will begin polling observed repositories for changes
    /// at the configured interval and trigger any configured AI agents.
    Start,

    /// Stop the running ghad daemon.
    ///
    /// Gracefully shuts down the background daemon process.
    Stop,

    /// Check the status of the ghad daemon.
    ///
    /// Shows whether the daemon is running, its PID, uptime,
    /// and last check timestamp.
    Status,

    /// Install the ghad daemon as a system service.
    ///
    /// On macOS, this creates a LaunchAgent plist.
    /// On Linux, this creates a systemd user service.
    Install,

    /// Uninstall the ghad daemon system service.
    ///
    /// Removes the system service configuration created by 'install'.
    Uninstall,

    /// Run the daemon in the foreground.
    #[command(hide = true)]
    Run {
        #[arg(long)]
        config_dir: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parses_no_args_shows_help() {
        // Without subcommand, should error (subcommand required)
        let result = Cli::try_parse_from(["ghad"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_debug_display() {
        let cli = Cli::try_parse_from(["ghad", "org", "list"]).unwrap();
        let debug_str = format!("{:?}", cli);
        assert!(debug_str.contains("Cli"));
    }

    #[test]
    fn test_verify_cli() {
        // Clap's built-in assertion to verify the CLI structure is valid
        Cli::command().debug_assert();
    }

    #[test]
    fn test_auth_configure() {
        let cli = Cli::try_parse_from(["ghad", "auth", "configure"]).unwrap();
        assert!(matches!(cli.command, Commands::Auth(AuthCommands::Configure)));
    }

    #[test]
    fn test_org_list() {
        let cli = Cli::try_parse_from(["ghad", "org", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::Org(OrgCommands::List)));
    }

    #[test]
    fn test_repo_list_no_filter() {
        let cli = Cli::try_parse_from(["ghad", "repo", "list"]).unwrap();
        match &cli.command {
            Commands::Repo(RepoCommands::List { org, all_orgs }) => {
                assert!(org.is_none());
                assert!(!all_orgs);
            }
            _ => panic!("Expected Repo List command"),
        }
    }

    #[test]
    fn test_repo_list_with_org() {
        let cli = Cli::try_parse_from(["ghad", "repo", "list", "--org", "mycompany"]).unwrap();
        match &cli.command {
            Commands::Repo(RepoCommands::List { org, all_orgs }) => {
                assert_eq!(org.as_deref(), Some("mycompany"));
                assert!(!all_orgs);
            }
            _ => panic!("Expected Repo List command"),
        }
    }

    #[test]
    fn test_repo_list_with_org_short() {
        let cli = Cli::try_parse_from(["ghad", "repo", "list", "-o", "mycompany"]).unwrap();
        match &cli.command {
            Commands::Repo(RepoCommands::List { org, all_orgs }) => {
                assert_eq!(org.as_deref(), Some("mycompany"));
                assert!(!all_orgs);
            }
            _ => panic!("Expected Repo List command"),
        }
    }

    #[test]
    fn test_repo_list_all_orgs() {
        let cli = Cli::try_parse_from(["ghad", "repo", "list", "--all-orgs"]).unwrap();
        match &cli.command {
            Commands::Repo(RepoCommands::List { org, all_orgs }) => {
                assert!(org.is_none());
                assert!(*all_orgs);
            }
            _ => panic!("Expected Repo List command"),
        }
    }

    #[test]
    fn test_project_list_no_filter() {
        let cli = Cli::try_parse_from(["ghad", "project", "list"]).unwrap();
        match &cli.command {
            Commands::Project(ProjectCommands::List { org }) => {
                assert!(org.is_none());
            }
            _ => panic!("Expected Project List command"),
        }
    }

    #[test]
    fn test_project_list_with_org() {
        let cli = Cli::try_parse_from(["ghad", "project", "list", "--org", "acme"]).unwrap();
        match &cli.command {
            Commands::Project(ProjectCommands::List { org }) => {
                assert_eq!(org.as_deref(), Some("acme"));
            }
            _ => panic!("Expected Project List command"),
        }
    }

    #[test]
    fn test_pr_list_no_filter() {
        let cli = Cli::try_parse_from(["ghad", "pr", "list"]).unwrap();
        match &cli.command {
            Commands::Pr(PrCommands::List { repo, org }) => {
                assert!(repo.is_none());
                assert!(org.is_none());
            }
            _ => panic!("Expected PR List command"),
        }
    }

    #[test]
    fn test_pr_list_with_repo_and_org() {
        let cli =
            Cli::try_parse_from(["ghad", "pr", "list", "--repo", "myrepo", "--org", "myorg"])
                .unwrap();
        match &cli.command {
            Commands::Pr(PrCommands::List { repo, org }) => {
                assert_eq!(repo.as_deref(), Some("myrepo"));
                assert_eq!(org.as_deref(), Some("myorg"));
            }
            _ => panic!("Expected PR List command"),
        }
    }

    #[test]
    fn test_issue_list_full_filters() {
        let cli = Cli::try_parse_from([
            "ghad",
            "issue",
            "list",
            "--repo",
            "myrepo",
            "--org",
            "myorg",
            "--project",
            "Sprint Board",
        ])
        .unwrap();
        match &cli.command {
            Commands::Issue(IssueCommands::List { repo, org, project }) => {
                assert_eq!(repo.as_deref(), Some("myrepo"));
                assert_eq!(org.as_deref(), Some("myorg"));
                assert_eq!(project.as_deref(), Some("Sprint Board"));
            }
            _ => panic!("Expected Issue List command"),
        }
    }

    #[test]
    fn test_issue_list_no_filters() {
        let cli = Cli::try_parse_from(["ghad", "issue", "list"]).unwrap();
        match &cli.command {
            Commands::Issue(IssueCommands::List { repo, org, project }) => {
                assert!(repo.is_none());
                assert!(org.is_none());
                assert!(project.is_none());
            }
            _ => panic!("Expected Issue List command"),
        }
    }

    #[test]
    fn test_observe_list() {
        let cli = Cli::try_parse_from(["ghad", "observe", "list"]).unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert!(matches!(
                    args.subcommand,
                    Some(ObserveSubcommands::List)
                ));
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_observe_repo_with_flags() {
        let cli = Cli::try_parse_from([
            "ghad",
            "observe",
            "myorg/myrepo",
            "--issues",
            "--prs",
            "--prompt",
            "Check for bugs",
            "--claude",
        ])
        .unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert_eq!(args.repo.as_deref(), Some("myorg/myrepo"));
                assert!(args.issues);
                assert!(args.prs);
                assert_eq!(args.prompt.as_deref(), Some("Check for bugs"));
                assert!(args.claude);
                assert!(!args.codex);
                assert!(!args.agy);
                assert!(!args.copilot);
                assert_eq!(args.agent(), Some("claude"));
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_observe_agent_mutual_exclusivity() {
        // Cannot specify both --claude and --codex
        let result = Cli::try_parse_from([
            "ghad",
            "observe",
            "myorg/myrepo",
            "--claude",
            "--codex",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_observe_agent_none() {
        let cli = Cli::try_parse_from(["ghad", "observe", "myorg/myrepo"]).unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert_eq!(args.agent(), None);
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_observe_agent_codex() {
        let cli =
            Cli::try_parse_from(["ghad", "observe", "myorg/myrepo", "--codex"]).unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert_eq!(args.agent(), Some("codex"));
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_observe_agent_agy() {
        let cli =
            Cli::try_parse_from(["ghad", "observe", "myorg/myrepo", "--agy"]).unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert_eq!(args.agent(), Some("agy"));
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_observe_agent_copilot() {
        let cli =
            Cli::try_parse_from(["ghad", "observe", "myorg/myrepo", "--copilot"]).unwrap();
        match &cli.command {
            Commands::Observe(args) => {
                assert_eq!(args.agent(), Some("copilot"));
            }
            _ => panic!("Expected Observe command"),
        }
    }

    #[test]
    fn test_prompt_global() {
        let cli =
            Cli::try_parse_from(["ghad", "prompt", "--global", "Check all the things"])
                .unwrap();
        match &cli.command {
            Commands::Prompt(args) => {
                assert!(args.global);
                assert_eq!(args.prompt, "Check all the things");
                assert!(args.repo.is_none());
                assert_eq!(args.scope(), PromptScope::All);
            }
            _ => panic!("Expected Prompt command"),
        }
    }

    #[test]
    fn test_prompt_repo_issue_scope() {
        let cli = Cli::try_parse_from([
            "ghad",
            "prompt",
            "--repo",
            "myorg/myrepo",
            "--issue",
            "Triage this",
        ])
        .unwrap();
        match &cli.command {
            Commands::Prompt(args) => {
                assert!(!args.global);
                assert_eq!(args.repo.as_deref(), Some("myorg/myrepo"));
                assert_eq!(args.scope(), PromptScope::Issue);
            }
            _ => panic!("Expected Prompt command"),
        }
    }

    #[test]
    fn test_prompt_repo_pr_scope() {
        let cli = Cli::try_parse_from([
            "ghad",
            "prompt",
            "--repo",
            "myorg/myrepo",
            "--pr",
            "Review quality",
        ])
        .unwrap();
        match &cli.command {
            Commands::Prompt(args) => {
                assert!(!args.global);
                assert_eq!(args.scope(), PromptScope::Pr);
            }
            _ => panic!("Expected Prompt command"),
        }
    }

    #[test]
    fn test_prompt_scope_mutual_exclusivity() {
        // Cannot specify both --issue and --pr
        let result = Cli::try_parse_from([
            "ghad", "prompt", "--issue", "--pr", "do stuff",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_daemon_start() {
        let cli = Cli::try_parse_from(["ghad", "daemon", "start"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommands::Start)
        ));
    }

    #[test]
    fn test_daemon_stop() {
        let cli = Cli::try_parse_from(["ghad", "daemon", "stop"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommands::Stop)
        ));
    }

    #[test]
    fn test_daemon_status() {
        let cli = Cli::try_parse_from(["ghad", "daemon", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommands::Status)
        ));
    }

    #[test]
    fn test_daemon_install() {
        let cli = Cli::try_parse_from(["ghad", "daemon", "install"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommands::Install)
        ));
    }

    #[test]
    fn test_daemon_uninstall() {
        let cli = Cli::try_parse_from(["ghad", "daemon", "uninstall"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Daemon(DaemonCommands::Uninstall)
        ));
    }

    #[test]
    fn test_verbose_flag() {
        let cli = Cli::try_parse_from(["ghad", "-vvv", "org", "list"]).unwrap();
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn test_format_json() {
        let cli =
            Cli::try_parse_from(["ghad", "--format", "json", "org", "list"]).unwrap();
        assert!(matches!(cli.format, OutputFormat::Json));
    }

    #[test]
    fn test_format_text_default() {
        let cli = Cli::try_parse_from(["ghad", "org", "list"]).unwrap();
        assert!(matches!(cli.format, OutputFormat::Text));
    }

    #[test]
    fn test_output_format_debug() {
        let f = OutputFormat::Text;
        let debug_str = format!("{:?}", f);
        assert_eq!(debug_str, "Text");
    }

    #[test]
    fn test_prompt_scope_debug() {
        assert_eq!(format!("{:?}", PromptScope::All), "All");
        assert_eq!(format!("{:?}", PromptScope::Issue), "Issue");
        assert_eq!(format!("{:?}", PromptScope::Pr), "Pr");
    }

    #[test]
    fn test_prompt_scope_eq() {
        assert_eq!(PromptScope::All, PromptScope::All);
        assert_ne!(PromptScope::Issue, PromptScope::Pr);
    }

    #[test]
    fn test_prompt_scope_clone() {
        let scope = PromptScope::Issue;
        let cloned = scope.clone();
        assert_eq!(scope, cloned);
    }
}

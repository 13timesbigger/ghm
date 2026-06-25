use colored::Colorize;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Maximum width for truncating long strings in table cells.
const MAX_CELL_WIDTH: usize = 50;

/// Truncate a string to the given max length, appending "…" if truncated.
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 1 {
        "…".to_string()
    } else {
        let mut end = max_len - 1;
        // Don't split in the middle of a multi-byte char
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Create a new spinner with the given message.
pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a base table with consistent styling.
fn base_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Create a styled header cell.
fn header(text: &str) -> Cell {
    Cell::new(text)
        .fg(Color::Cyan)
        .add_attribute(Attribute::Bold)
}

/// Color-code a status string.
pub fn colored_status(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "open" => status.green().bold().to_string(),
        "closed" => status.red().to_string(),
        "merged" => status.purple().bold().to_string(),
        "active" | "running" => status.green().to_string(),
        "stopped" | "inactive" => status.yellow().to_string(),
        _ => status.to_string(),
    }
}

// ── Organization Table ────────────────────────────────────────────────

/// Row data for the organization table.
pub struct OrgRow {
    pub login: String,
    pub description: String,
}

/// Render a table of organizations.
pub fn print_orgs_table(orgs: &[OrgRow]) {
    if orgs.is_empty() {
        println!("{}", "No organizations found.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![header("#"), header("Login"), header("Description")]);

    for (i, org) in orgs.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(&org.login).fg(Color::White),
            Cell::new(truncate(&org.description, MAX_CELL_WIDTH)),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} organization(s)", orgs.len()).dimmed()
    );
}

// ── Repository Table ──────────────────────────────────────────────────

/// Row data for the repository table.
pub struct RepoRow {
    pub full_name: String,
    pub description: String,
    pub language: String,
    pub stars: u64,
    pub updated_at: String,
}

/// Render a table of repositories.
pub fn print_repos_table(repos: &[RepoRow]) {
    if repos.is_empty() {
        println!("{}", "No repositories found.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Repository"),
        header("Description"),
        header("Language"),
        header("Stars"),
        header("Updated"),
    ]);

    for (i, repo) in repos.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(&repo.full_name).fg(Color::White),
            Cell::new(truncate(&repo.description, MAX_CELL_WIDTH)),
            Cell::new(&repo.language).fg(Color::Yellow),
            Cell::new(repo.stars).set_alignment(CellAlignment::Right),
            Cell::new(&repo.updated_at),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} repository(ies)", repos.len()).dimmed()
    );
}

// ── Project Table ─────────────────────────────────────────────────────

/// Row data for the project table.
pub struct ProjectRow {
    pub title: String,
    pub description: String,
    pub status: String,
    pub item_count: u64,
}

/// Render a table of projects.
pub fn print_projects_table(projects: &[ProjectRow]) {
    if projects.is_empty() {
        println!("{}", "No projects found.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Title"),
        header("Description"),
        header("Status"),
        header("Items"),
    ]);

    for (i, proj) in projects.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(&proj.title).fg(Color::White),
            Cell::new(truncate(&proj.description, MAX_CELL_WIDTH)),
            Cell::new(colored_status(&proj.status)),
            Cell::new(proj.item_count).set_alignment(CellAlignment::Right),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} project(s)", projects.len()).dimmed()
    );
}

// ── Pull Request Table ────────────────────────────────────────────────

/// Row data for the pull request table.
pub struct PrRow {
    pub number: u64,
    pub title: String,
    pub status: String,
    pub author: String,
    pub repo: String,
    pub created_at: String,
}

/// Render a table of pull requests.
pub fn print_prs_table(prs: &[PrRow]) {
    if prs.is_empty() {
        println!("{}", "No pull requests found.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Number"),
        header("Title"),
        header("Status"),
        header("Author"),
        header("Repository"),
        header("Created"),
    ]);

    for (i, pr) in prs.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(format!("#{}", pr.number)).fg(Color::Cyan),
            Cell::new(truncate(&pr.title, MAX_CELL_WIDTH)),
            Cell::new(colored_status(&pr.status)),
            Cell::new(&pr.author),
            Cell::new(&pr.repo).fg(Color::DarkGrey),
            Cell::new(&pr.created_at),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} pull request(s)", prs.len()).dimmed()
    );
}

// ── Issue Table ───────────────────────────────────────────────────────

/// Row data for the issue table.
pub struct IssueRow {
    pub number: u64,
    pub title: String,
    pub status: String,
    pub labels: String,
    pub assignee: String,
    pub repo: String,
    pub created_at: String,
}

/// Render a table of issues.
pub fn print_issues_table(issues: &[IssueRow]) {
    if issues.is_empty() {
        println!("{}", "No issues found.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Number"),
        header("Title"),
        header("Status"),
        header("Labels"),
        header("Assignee"),
        header("Repository"),
        header("Created"),
    ]);

    for (i, issue) in issues.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(format!("#{}", issue.number)).fg(Color::Cyan),
            Cell::new(truncate(&issue.title, MAX_CELL_WIDTH)),
            Cell::new(colored_status(&issue.status)),
            Cell::new(truncate(&issue.labels, 30)).fg(Color::Yellow),
            Cell::new(&issue.assignee),
            Cell::new(&issue.repo).fg(Color::DarkGrey),
            Cell::new(&issue.created_at),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} issue(s)", issues.len()).dimmed()
    );
}

// ── Observed Repos Table ──────────────────────────────────────────────

/// Row data for the observed repositories table.
pub struct ObservedRow {
    pub repo: String,
    pub watch_issues: bool,
    pub watch_prs: bool,
    pub agent: String,
    pub prompt: String,
}

/// Render a table of observed repositories.
pub fn print_observed_table(observed: &[ObservedRow]) {
    if observed.is_empty() {
        println!(
            "{}",
            "No observed repositories. Use 'ghm observe <repo>' to start watching."
                .yellow()
        );
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Repository"),
        header("Issues"),
        header("PRs"),
        header("Agent"),
        header("Prompt"),
    ]);

    for (i, obs) in observed.iter().enumerate() {
        let issues_str = if obs.watch_issues { "✓" } else { "—" };
        let prs_str = if obs.watch_prs { "✓" } else { "—" };

        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(&obs.repo).fg(Color::White),
            Cell::new(issues_str).fg(if obs.watch_issues {
                Color::Green
            } else {
                Color::DarkGrey
            }),
            Cell::new(prs_str).fg(if obs.watch_prs {
                Color::Green
            } else {
                Color::DarkGrey
            }),
            Cell::new(&obs.agent),
            Cell::new(truncate(&obs.prompt, MAX_CELL_WIDTH)),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} observed repository(ies)", observed.len()).dimmed()
    );
}

// ── Prompt Table ──────────────────────────────────────────────────────

/// Row data for the prompt display.
pub struct PromptRow {
    pub scope: String,
    pub repo: String,
    pub event_type: String,
    pub prompt: String,
}

/// Render a table of configured prompts.
pub fn print_prompts_table(prompts: &[PromptRow]) {
    if prompts.is_empty() {
        println!("{}", "No prompts configured.".yellow());
        return;
    }

    let mut table = base_table();
    table.set_header(vec![
        header("#"),
        header("Scope"),
        header("Repository"),
        header("Event Type"),
        header("Prompt"),
    ]);

    for (i, p) in prompts.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(CellAlignment::Right),
            Cell::new(&p.scope).fg(Color::Cyan),
            Cell::new(&p.repo),
            Cell::new(&p.event_type),
            Cell::new(truncate(&p.prompt, MAX_CELL_WIDTH)),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        format!("Total: {} prompt(s)", prompts.len()).dimmed()
    );
}

// ── Daemon Status ─────────────────────────────────────────────────────

/// Display daemon status information.
pub fn print_daemon_status(running: bool, pid: Option<u32>, uptime: Option<&str>) {
    let status_str = if running {
        "running".green().bold().to_string()
    } else {
        "stopped".red().to_string()
    };

    println!("Daemon status: {}", status_str);
    if let Some(pid) = pid {
        println!("PID: {}", pid.to_string().cyan());
    }
    if let Some(uptime) = uptime {
        println!("Uptime: {}", uptime.dimmed());
    }
}

/// Print a success message.
pub fn print_success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

/// Print an error message.
pub fn print_error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

/// Print an info message.
pub fn print_info(msg: &str) {
    println!("{} {}", "ℹ".cyan(), msg);
}

/// Print a warning message.
pub fn print_warning(msg: &str) {
    println!("{} {}", "⚠".yellow(), msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate tests ────────────────────────────────────────────────

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("hello world this is long", 10);
        assert_eq!(result, "hello wor…");
        assert!(result.len() <= 12); // "…" is 3 bytes in UTF-8
    }

    #[test]
    fn test_truncate_zero_max() {
        assert_eq!(truncate("hello", 0), "…");
    }

    #[test]
    fn test_truncate_one_max() {
        assert_eq!(truncate("hello", 1), "…");
    }

    #[test]
    fn test_truncate_empty_string() {
        assert_eq!(truncate("", 10), "");
    }

    #[test]
    fn test_truncate_unicode() {
        // Each emoji is 4 bytes; ensure we don't split mid-character
        let input = "🎉🎊🎈🎆";
        let result = truncate(input, 5);
        // Should truncate at a char boundary
        assert!(result.ends_with('…'));
    }

    // ── colored_status tests ──────────────────────────────────────────

    #[test]
    fn test_colored_status_open() {
        let result = colored_status("open");
        // The result contains ANSI escape codes, but should contain "open"
        assert!(result.contains("open"));
    }

    #[test]
    fn test_colored_status_closed() {
        let result = colored_status("closed");
        assert!(result.contains("closed"));
    }

    #[test]
    fn test_colored_status_merged() {
        let result = colored_status("merged");
        assert!(result.contains("merged"));
    }

    #[test]
    fn test_colored_status_active() {
        let result = colored_status("active");
        assert!(result.contains("active"));
    }

    #[test]
    fn test_colored_status_stopped() {
        let result = colored_status("stopped");
        assert!(result.contains("stopped"));
    }

    #[test]
    fn test_colored_status_unknown() {
        let result = colored_status("something_else");
        assert_eq!(result, "something_else");
    }

    #[test]
    fn test_colored_status_case_insensitive() {
        let result = colored_status("OPEN");
        assert!(result.contains("OPEN"));
    }

    // ── spinner test ──────────────────────────────────────────────────

    #[test]
    fn test_spinner_creation() {
        let pb = spinner("Loading...");
        assert!(!pb.is_finished());
        pb.finish_and_clear();
        assert!(pb.is_finished());
    }

    // ── Row struct construction tests ─────────────────────────────────

    #[test]
    fn test_org_row_construction() {
        let row = OrgRow {
            login: "myorg".to_string(),
            description: "My organization".to_string(),
        };
        assert_eq!(row.login, "myorg");
        assert_eq!(row.description, "My organization");
    }

    #[test]
    fn test_repo_row_construction() {
        let row = RepoRow {
            full_name: "myorg/myrepo".to_string(),
            description: "A repo".to_string(),
            language: "Rust".to_string(),
            stars: 42,
            updated_at: "2024-01-15".to_string(),
        };
        assert_eq!(row.full_name, "myorg/myrepo");
        assert_eq!(row.stars, 42);
    }

    #[test]
    fn test_project_row_construction() {
        let row = ProjectRow {
            title: "Sprint Board".to_string(),
            description: "Main project board".to_string(),
            status: "open".to_string(),
            item_count: 15,
        };
        assert_eq!(row.title, "Sprint Board");
        assert_eq!(row.item_count, 15);
    }

    #[test]
    fn test_pr_row_construction() {
        let row = PrRow {
            number: 123,
            title: "Fix bug".to_string(),
            status: "open".to_string(),
            author: "dev1".to_string(),
            repo: "myorg/myrepo".to_string(),
            created_at: "2024-01-15".to_string(),
        };
        assert_eq!(row.number, 123);
        assert_eq!(row.status, "open");
    }

    #[test]
    fn test_issue_row_construction() {
        let row = IssueRow {
            number: 456,
            title: "Feature request".to_string(),
            status: "open".to_string(),
            labels: "enhancement, feature".to_string(),
            assignee: "dev2".to_string(),
            repo: "myorg/myrepo".to_string(),
            created_at: "2024-01-15".to_string(),
        };
        assert_eq!(row.number, 456);
        assert_eq!(row.labels, "enhancement, feature");
    }

    #[test]
    fn test_observed_row_construction() {
        let row = ObservedRow {
            repo: "myorg/myrepo".to_string(),
            watch_issues: true,
            watch_prs: false,
            agent: "claude".to_string(),
            prompt: "Review for security".to_string(),
        };
        assert!(row.watch_issues);
        assert!(!row.watch_prs);
        assert_eq!(row.agent, "claude");
    }

    #[test]
    fn test_prompt_row_construction() {
        let row = PromptRow {
            scope: "global".to_string(),
            repo: "*".to_string(),
            event_type: "all".to_string(),
            prompt: "Always check for bugs".to_string(),
        };
        assert_eq!(row.scope, "global");
        assert_eq!(row.prompt, "Always check for bugs");
    }

    // ── Table rendering tests (smoke tests — ensure no panics) ────────

    #[test]
    fn test_print_orgs_table_empty() {
        // Should not panic
        print_orgs_table(&[]);
    }

    #[test]
    fn test_print_orgs_table_with_data() {
        let orgs = vec![
            OrgRow {
                login: "org1".to_string(),
                description: "First org".to_string(),
            },
            OrgRow {
                login: "org2".to_string(),
                description: "Second org with a very long description that should be truncated at some point because it is excessively verbose".to_string(),
            },
        ];
        // Should not panic
        print_orgs_table(&orgs);
    }

    #[test]
    fn test_print_repos_table_empty() {
        print_repos_table(&[]);
    }

    #[test]
    fn test_print_repos_table_with_data() {
        let repos = vec![RepoRow {
            full_name: "org/repo".to_string(),
            description: "A test repo".to_string(),
            language: "Rust".to_string(),
            stars: 100,
            updated_at: "2024-06-01".to_string(),
        }];
        print_repos_table(&repos);
    }

    #[test]
    fn test_print_projects_table_empty() {
        print_projects_table(&[]);
    }

    #[test]
    fn test_print_projects_table_with_data() {
        let projects = vec![ProjectRow {
            title: "Sprint 1".to_string(),
            description: "Current sprint".to_string(),
            status: "open".to_string(),
            item_count: 10,
        }];
        print_projects_table(&projects);
    }

    #[test]
    fn test_print_prs_table_empty() {
        print_prs_table(&[]);
    }

    #[test]
    fn test_print_prs_table_with_data() {
        let prs = vec![PrRow {
            number: 1,
            title: "Add feature".to_string(),
            status: "merged".to_string(),
            author: "dev".to_string(),
            repo: "org/repo".to_string(),
            created_at: "2024-01-01".to_string(),
        }];
        print_prs_table(&prs);
    }

    #[test]
    fn test_print_issues_table_empty() {
        print_issues_table(&[]);
    }

    #[test]
    fn test_print_issues_table_with_data() {
        let issues = vec![IssueRow {
            number: 42,
            title: "Bug report".to_string(),
            status: "open".to_string(),
            labels: "bug".to_string(),
            assignee: "nobody".to_string(),
            repo: "org/repo".to_string(),
            created_at: "2024-03-15".to_string(),
        }];
        print_issues_table(&issues);
    }

    #[test]
    fn test_print_observed_table_empty() {
        print_observed_table(&[]);
    }

    #[test]
    fn test_print_observed_table_with_data() {
        let observed = vec![ObservedRow {
            repo: "org/repo".to_string(),
            watch_issues: true,
            watch_prs: true,
            agent: "claude".to_string(),
            prompt: "Review everything".to_string(),
        }];
        print_observed_table(&observed);
    }

    #[test]
    fn test_print_prompts_table_empty() {
        print_prompts_table(&[]);
    }

    #[test]
    fn test_print_prompts_table_with_data() {
        let prompts = vec![PromptRow {
            scope: "global".to_string(),
            repo: "*".to_string(),
            event_type: "all".to_string(),
            prompt: "Check everything".to_string(),
        }];
        print_prompts_table(&prompts);
    }

    #[test]
    fn test_print_daemon_status_running() {
        // Should not panic
        print_daemon_status(true, Some(12345), Some("2h 30m"));
    }

    #[test]
    fn test_print_daemon_status_stopped() {
        print_daemon_status(false, None, None);
    }

    #[test]
    fn test_print_success() {
        print_success("Operation completed");
    }

    #[test]
    fn test_print_error() {
        print_error("Something went wrong");
    }

    #[test]
    fn test_print_info() {
        print_info("Here is some info");
    }

    #[test]
    fn test_print_warning() {
        print_warning("Watch out");
    }
}

# Github Activity to Agents Dispatcher (GHAAD)

Github Activity to Agents Dispatcher (GHAAD) is a background daemon and CLI tool for observing GitHub repositories for new pull requests and issues, and dispatching those activities to AI agents.

## Features
- **CLI Management**: Authenticate, list organizations, repositories, projects, pull requests, and issues.
- **Observation Engine**: Add repositories to a watch list to monitor issues and PRs.
- **AI Agents**: Configure prompts and AI agents (Codex, Agy, Claude, Copilot) to automatically process and respond to new events.
- **Daemon**: Runs in the background on macOS (via `launchd`) to poll the GitHub API based on your configured watch list.

## Installation

Install the `ghad` binary for GHAAD into `/usr/local/bin`:
```bash
./install.sh
```

Or install directly from GitHub:
```bash
curl -fsSL https://raw.githubusercontent.com/corelmax/github-monitor/main/install.sh | sh
```

To install somewhere else, pass `--dir` or set `INSTALL_DIR`:
```bash
./install.sh --dir "$HOME/.local/bin"
curl -fsSL https://raw.githubusercontent.com/corelmax/github-monitor/main/install.sh | sh -s -- --dir "$HOME/.local/bin"
```

You can also build the binary manually using Cargo:
```bash
cargo build --release
```

The binary will be located at `target/release/ghad`.

## Usage

### Authentication
Before doing anything else, authenticate with GitHub:
```bash
ghad auth configure
```
This will save your configuration to `~/.config/ghad/config.json`.

`ghad auth configure` supports GitHub App installation auth only. Create a GitHub
App with the repository permissions GHAAD needs, then run `ghad auth configure`.
The command opens the GitHub App installation page in your default browser so you
can choose the account or organization and either all repositories or selected
repositories.

You can prefill the interactive prompts with environment variables:
```bash
export GHAD_GITHUB_APP_SLUG="your-app-slug"
export GHAD_GITHUB_APP_ID="123456"
export GHAD_GITHUB_APP_PRIVATE_KEY_PATH="$HOME/.config/ghad/private-key.pem"
export GHAD_GITHUB_APP_INSTALLATION_ID="987654"
ghad auth configure
```

### Listing Resources
You can list resources from GitHub using the CLI:
```bash
ghad org list
ghad repo list
ghad repo list --all-orgs
ghad repo list --org myorganization
ghad project list
ghad pr list --repo myorg/myrepo
ghad issue list --org myorg
```

With GitHub App installation auth, `ghad repo list` shows repositories granted to
the selected installation.

### Observation and Agents
To add a repository to the watch list:
```bash
ghad observe myorg/myrepo --issues --prs --working-dir /path/to/myrepo --prompt "Check for bugs" --claude
```

List your observed repositories:
```bash
ghad observe list
```

When an agent is configured for an observed repository, GHAAD runs the agent from
that repository's `--working-dir`. If no repository working directory is set, it
falls back to the global configured working directory, then the GHAAD config
directory.

Manage default prompts for the agents:
```bash
# Global prompt
ghad prompt --global "Always be polite"

# Repository-specific prompt
ghad prompt --repo myorg/myrepo --pr "Review this PR thoroughly"
```

### Daemon Management
The daemon runs in the background and polls the GitHub API for your observed repositories.

```bash
# Start the daemon in the foreground
ghad daemon start

# Install the daemon as a background service (macOS launchd)
ghad daemon install

# Check status
ghad daemon status

# Uninstall the background service
ghad daemon uninstall
```

## Architecture
- `ghad-core`: The library handling GitHub API interactions (via Octocrab), state persistence, configuration, and the daemon polling engine.
- `ghad-cli`: The command-line interface handling argument parsing (via Clap) and formatted table output (via Comfy-Table).

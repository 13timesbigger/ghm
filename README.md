# GitHub Monitor (ghm)

A background daemon and CLI tool for observing GitHub repositories for new pull requests and issues, and reacting to them using AI agents.

## Features
- **CLI Management**: Authenticate, list organizations, repositories, projects, pull requests, and issues.
- **Observation Engine**: Add repositories to a watch list to monitor issues and PRs.
- **AI Agents**: Configure prompts and AI agents (Codex, Agy, Claude, Copilot) to automatically process and respond to new events.
- **Daemon**: Runs in the background on macOS (via `launchd`) to poll the GitHub API based on your configured watch list.

## Installation

Build the binary using Cargo:
```bash
cargo build --release
```

The binary will be located at `target/release/ghm`. You may want to copy it to your `PATH` (e.g., `/usr/local/bin`).

## Usage

### Authentication
Before doing anything else, authenticate with GitHub using a Personal Access Token (PAT):
```bash
ghm auth configure
```
This will save your configuration to `~/.config/ghm/config.json`.

### Listing Resources
You can list resources from GitHub using the CLI:
```bash
ghm org list
ghm repo list --org myorganization
ghm project list
ghm pr list --repo myorg/myrepo
ghm issue list --org myorg
```

### Observation and Agents
To add a repository to the watch list:
```bash
ghm observe myorg/myrepo --issues --prs --prompt "Check for bugs" --claude
```

List your observed repositories:
```bash
ghm observe list
```

Manage default prompts for the agents:
```bash
# Global prompt
ghm prompt --global "Always be polite"

# Repository-specific prompt
ghm prompt --repo myorg/myrepo --pr "Review this PR thoroughly"
```

### Daemon Management
The daemon runs in the background and polls the GitHub API for your observed repositories.

```bash
# Start the daemon in the foreground
ghm daemon start

# Install the daemon as a background service (macOS launchd)
ghm daemon install

# Check status
ghm daemon status

# Uninstall the background service
ghm daemon uninstall
```

## Architecture
- `ghm-core`: The library handling GitHub API interactions (via Octocrab), state persistence, configuration, and the daemon polling engine.
- `ghm-cli`: The command-line interface handling argument parsing (via Clap) and formatted table output (via Comfy-Table).

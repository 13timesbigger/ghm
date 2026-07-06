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
Before doing anything else, authenticate with GitHub using a Personal Access Token (PAT):
```bash
ghad auth configure
```
This will save your configuration to `~/.config/ghad/config.json`.

GitHub Device Flow is also supported. Select it during `ghad auth configure`, then enter a GitHub OAuth app client ID when prompted or set `GHAD_GITHUB_CLIENT_ID` before running the command.

### Listing Resources
You can list resources from GitHub using the CLI:
```bash
ghad org list
ghad repo list --org myorganization
ghad project list
ghad pr list --repo myorg/myrepo
ghad issue list --org myorg
```

### Observation and Agents
To add a repository to the watch list:
```bash
ghad observe myorg/myrepo --issues --prs --prompt "Check for bugs" --claude
```

List your observed repositories:
```bash
ghad observe list
```

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

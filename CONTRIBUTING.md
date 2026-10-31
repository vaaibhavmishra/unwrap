# Contributing to Unwrap

Thank you for your interest in contributing to Unwrap! We welcome community contributions, bug fixes, and improvements.

## Code of Conduct

Please be respectful and constructive in all discussions and interactions.

## How to Contribute

### 1. Reporting Bugs

Before reporting a bug, please search existing issues to see if it has already been reported.

When opening an issue, include:
- A concise summary of the issue.
- Steps to reproduce the bug.
- Expected vs. actual behavior.
- Operating system and version (e.g. macOS 14.5, Ubuntu 24.04).
- Any relevant logs or screenshots.

### 2. Proposing Changes

For larger architectural changes or new features, please open an issue first to discuss the design and approach before submitting a pull request.

### 3. Local Development

To set up your development environment:

```bash
# Clone the repository
git clone https://github.com/your-username/unwrap.git
cd unwrap

# Bootstrap build dependencies
./script/bootstrap

# Build and run the desktop app
./script/run

# Build and run the headless console TUI
./script/run-tui
```

### 4. Code Style & Verification

Before submitting a PR, ensure your changes format cleanly and pass checks:

```bash
# Format Rust code
./script/format

# Run linter
cargo clippy -p warp --all-targets --tests -- -D warnings

# Run targeted tests
cargo nextest run -p <package>
```

### 5. Submitting a Pull Request

1. Fork the repository and create your branch from `main`.
2. Keep pull requests focused on a single change or fix.
3. Include relevant unit or integration tests for new functionality.
4. Open the pull request with a descriptive title and summary of changes.

Thank you for helping make Unwrap better!

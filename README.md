<h1 align="center"><code>mq-update</code></h1>

Updater for `mq` and `mq` subcommands (`mq-check`, `mq-conv`, etc.).

![demo](assets/demo.gif)

## Overview

`mq-update` is a utility to update the `mq` binary and its subcommands (e.g., `mq-check`, `mq-conv`, `mq-docs`) to the latest version from GitHub releases.

## Installation

### Using the install script (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/harehare/mq-update/main/scripts/install.sh | bash
```

### From source with Cargo

```bash
cargo install --git https://github.com/harehare/mq-update.git
```

## Usage

### Update mq to the latest version

```bash
mq-update
```

### Update a subcommand

```bash
# Update mq-check to the latest version
mq-update check

# Update mq-conv to the latest version
mq-update conv
```

### Update to a specific version

```bash
mq-update --target v0.5.12
# or
mq-update --target 0.5.12

# Subcommand with a specific version
mq-update check --target v0.1.0
```

### Show current version

```bash
mq-update --current

# Subcommand version
mq-update check --current
```

### Force reinstall

```bash
mq-update --force
```

## Options

- `[SUBCOMMAND]`: Subcommand name to update (e.g., `check` for `mq-check`)
- `-t, --target <VERSION>`: Target version to install (defaults to latest)
- `-f, --force`: Force reinstall even if already up-to-date
- `--current`: Show current version
- `-h, --help`: Print help
- `-V, --version`: Print version

## How it works

1. Locates the binary (`mq` or `mq-{subcommand}`) via `which`
2. Checks the current version
3. Fetches the latest release information from GitHub (`harehare/mq` or `harehare/mq-{subcommand}`)
4. Downloads the appropriate binary for your platform
5. Creates a backup of the existing binary
6. Replaces the existing binary with the new one

## Supported Platforms

- **Linux** (glibc)
  - x86_64
  - aarch64
- **Linux** (musl) - Alpine Linux, etc.
  - x86_64
  - aarch64
- **macOS**
  - x86_64 (Intel)
  - aarch64 (Apple Silicon)
- **Windows**
  - x86_64

## License

MIT License - see [LICENSE](LICENSE) for details.

<h1 align="center"><code>mq-update</code></h1>

Updater for the `mq` command-line tool.

![demo](assets/demo.gif)

## Overview

`mq-update` is a utility to update the `mq` binary to the latest version from GitHub releases.

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

### Update to the latest version

```bash
mq-update
```

### Update to a specific version

```bash
mq-update --target v0.5.12
# or
mq-update --target 0.5.12
```

### Show current version

```bash
mq-update --current
```

### Force reinstall

```bash
mq-update --force
```

## Options

- `-t, --target <VERSION>`: Target version to install (defaults to latest)
- `-f, --force`: Force reinstall even if already up-to-date
- `--current`: Show current version
- `-h, --help`: Print help
- `-V, --version`: Print version

## How it works

1. Checks the current version of `mq`
2. Fetches the latest release information from GitHub
3. Downloads the appropriate binary for your platform
4. Creates a backup of the existing binary
5. Replaces the existing binary with the new one
6. Verifies the installation

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

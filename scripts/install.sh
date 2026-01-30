#!/usr/bin/env bash

set -e

# mq-update installation script

readonly MQ_UPDATE_REPO="harehare/mq-update"
readonly MQ_INSTALL_DIR="$HOME/.mq"
readonly MQ_BIN_DIR="$MQ_INSTALL_DIR/bin"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly PURPLE='\033[0;35m'
readonly CYAN='\033[0;36m'
readonly BOLD='\033[1m'
readonly NC='\033[0m' # No Color

# Utility functions
log() {
    echo -e "${GREEN}ℹ${NC}  $*" >&2
}

warn() {
    echo -e "${YELLOW}⚠${NC}  $*" >&2
}

error() {
    echo -e "${RED}✗${NC}  $*" >&2
    exit 1
}

# Display the mq-update logo
show_logo() {
    cat << 'EOF'

    ███╗   ███╗  ██████╗       ██╗   ██╗██████╗ ██████╗  █████╗ ████████╗███████╗
    ████╗ ████║ ██╔═══██╗      ██║   ██║██╔══██╗██╔══██╗██╔══██╗╚══██╔══╝██╔════╝
    ██╔████╔██║ ██║   ██║█████╗██║   ██║██████╔╝██║  ██║███████║   ██║   █████╗
    ██║╚██╔╝██║ ██║▄▄ ██║╚════╝██║   ██║██╔═══╝ ██║  ██║██╔══██║   ██║   ██╔══╝
    ██║ ╚═╝ ██║ ╚██████╔╝      ╚██████╔╝██║     ██████╔╝██║  ██║   ██║   ███████╗
    ╚═╝     ╚═╝  ╚══▀▀═╝        ╚═════╝ ╚═╝     ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝

EOF
    echo -e "${BOLD}${CYAN}                    mq Update Manager${NC}"
    echo -e "${BLUE}              Keep your mq installation up to date${NC}"
    echo ""
    echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Detect the operating system
detect_os() {
    case "$(uname -s)" in
        Linux*)
            echo "linux"
            ;;
        Darwin*)
            echo "darwin"
            ;;
        CYGWIN*|MINGW*|MSYS*)
            echo "windows"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
}

# Detect the architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
}

# Detect libc (musl or gnu)
detect_libc() {
    local os="$1"
    
    if [[ "$os" != "linux" ]]; then
        echo "gnu"
        return
    fi
    
    # Check if ldd exists and contains musl
    if command -v ldd &> /dev/null; then
        if ldd --version 2>&1 | grep -qi musl; then
            echo "musl"
            return
        fi
    fi
    
    # Check if musl-gcc exists
    if command -v musl-gcc &> /dev/null; then
        echo "musl"
        return
    fi
    
    # Default to gnu
    echo "gnu"
}

# Get the latest release version from GitHub
get_latest_version() {
    local version
    version=$(curl -s "https://api.github.com/repos/$MQ_UPDATE_REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        error "Failed to get the latest version"
    fi

    echo "$version"
}

# Construct the download URL for the binary
get_download_url() {
    local version="$1"
    local os="$2"
    local arch="$3"
    local libc="$4"
    local ext=""
    local target

    if [[ "$os" == "windows" ]]; then
        ext=".exe"
        target="${arch}-pc-windows-msvc"
    elif [[ "$os" == "darwin" ]]; then
        target="${arch}-apple-darwin"
    else
        if [[ "$libc" == "musl" ]]; then
            target="${arch}-unknown-linux-musl"
        else
            target="${arch}-unknown-linux-gnu"
        fi
    fi

    echo "https://github.com/$MQ_UPDATE_REPO/releases/download/$version/mq-update-${target}${ext}"
}

# Download and install mq-update
install_mq_update() {
    local version="$1"
    local os="$2"
    local arch="$3"
    local libc="$4"
    local download_url
    local binary_name="mq-update"
    local temp_file

    if [[ "$os" == "windows" ]]; then
        binary_name="mq-update.exe"
    fi

    download_url=$(get_download_url "$version" "$os" "$arch" "$libc")

    log "Downloading mq-update from $download_url"

    temp_file=$(mktemp)
    if ! curl -L --progress-bar "$download_url" -o "$temp_file"; then
        error "Failed to download mq-update"
    fi

    # Create installation directory
    mkdir -p "$MQ_BIN_DIR"

    # Move binary to installation directory
    local install_path="$MQ_BIN_DIR/$binary_name"
    mv "$temp_file" "$install_path"
    chmod +x "$install_path"

    log "✓ mq-update installed to $install_path"
}

# Update shell profile to add to PATH
update_shell_profile() {
    local shell_name
    shell_name=$(basename "$SHELL")

    # Skip if already in PATH
    if echo "$PATH" | grep -q "$MQ_BIN_DIR"; then
        log "✓ $MQ_BIN_DIR is already in PATH"
        return 0
    fi

    local shell_profile=""

    case "$shell_name" in
        bash)
            if [[ -f "$HOME/.bashrc" ]]; then
                shell_profile="$HOME/.bashrc"
            elif [[ -f "$HOME/.bash_profile" ]]; then
                shell_profile="$HOME/.bash_profile"
            fi
            ;;
        zsh)
            if [[ -f "$HOME/.zshrc" ]]; then
                shell_profile="$HOME/.zshrc"
            fi
            ;;
        fish)
            if [[ -d "$HOME/.config/fish" ]]; then
                shell_profile="$HOME/.config/fish/config.fish"
                mkdir -p "$(dirname "$shell_profile")"
            fi
            ;;
    esac

    if [[ -n "$shell_profile" ]]; then
        local path_export
        if [[ "$shell_name" == "fish" ]]; then
            path_export="set -gx PATH \$PATH $MQ_BIN_DIR"
        else
            path_export="export PATH=\"\$PATH:$MQ_BIN_DIR\""
        fi

        if ! grep -q "$MQ_BIN_DIR" "$shell_profile" 2>/dev/null; then
            echo "" >> "$shell_profile"
            echo "# Added by mq-update installer" >> "$shell_profile"
            echo "$path_export" >> "$shell_profile"
            log "Added $MQ_BIN_DIR to PATH in $shell_profile"
        else
            warn "$MQ_BIN_DIR already exists in $shell_profile"
        fi
    else
        warn "Could not detect shell profile to update"
        warn "Please manually add $MQ_BIN_DIR to your PATH"
    fi
}

# Verify installation
verify_installation() {
    local os="$1"
    local binary_name="mq-update"
    
    if [[ "$os" == "windows" ]]; then
        binary_name="mq-update.exe"
    fi

    if [[ -x "$MQ_BIN_DIR/$binary_name" ]]; then
        log "✓ mq-update installation verified"
        return 0
    else
        error "mq-update installation verification failed"
    fi
}

# Show post-installation instructions
show_post_install() {
    echo ""
    echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${GREEN}✨ mq-update installed successfully! ✨${NC}"
    echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}🚀 Getting Started:${NC}"
    echo ""
    echo -e "  ${YELLOW}1.${NC} Restart your terminal or run:"
    echo -e "     ${CYAN}source ~/.bashrc${NC} ${BLUE}(or your shell's profile)${NC}"
    echo ""
    echo -e "  ${YELLOW}2.${NC} Verify the installation:"
    echo -e "     ${CYAN}mq-update --version${NC}"
    echo ""
    echo -e "  ${YELLOW}3.${NC} Update mq to the latest version:"
    echo -e "     ${CYAN}mq-update${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}⚡ Usage Examples:${NC}"
    echo -e "  ${GREEN}▶${NC} ${CYAN}mq-update${NC}                    ${BLUE}# Update to latest version${NC}"
    echo -e "  ${GREEN}▶${NC} ${CYAN}mq-update --target v0.5.12${NC}   ${BLUE}# Update to specific version${NC}"
    echo -e "  ${GREEN}▶${NC} ${CYAN}mq-update --current${NC}          ${BLUE}# Show current mq version${NC}"
    echo -e "  ${GREEN}▶${NC} ${CYAN}mq-update --force${NC}            ${BLUE}# Force reinstall${NC}"
    echo ""
    echo -e "${BOLD}${CYAN}📚 Learn More:${NC}"
    echo -e "  ${GREEN}▶${NC} Get help:      ${CYAN}mq-update --help${NC}"
    echo -e "  ${GREEN}▶${NC} Repository:    ${BLUE}https://github.com/$MQ_UPDATE_REPO${NC}"
    echo -e "  ${GREEN}▶${NC} Main project:  ${BLUE}https://github.com/harehare/mq${NC}"
    echo ""
    echo -e "${PURPLE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""
}

# Main installation function
main() {
    show_logo

    # Check if curl is available
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed"
    fi

    # Detect system
    local os arch libc version
    os=$(detect_os)
    arch=$(detect_arch)
    libc=$(detect_libc "$os")

    log "Detected system: $os/$arch/$libc"

    # Get latest version
    version=$(get_latest_version)
    log "Latest version: $version"

    # Install mq-update
    install_mq_update "$version" "$os" "$arch" "$libc"

    # Update shell profile
    update_shell_profile

    # Verify installation
    verify_installation "$os"

    # Show post-installation instructions
    show_post_install
}

# Handle script arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "mq-update installation script"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --help, -h        Show this help message"
            echo "  --version, -v     Show version and exit"
            exit 0
            ;;
        --version|-v)
            echo "mq-update installer v1.0.0"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            ;;
    esac
    shift
done

# Check if we're running in a supported environment
if [[ -z "${BASH_VERSION:-}" ]]; then
    error "This installer requires bash"
fi

# Run the main installation
main "$@"

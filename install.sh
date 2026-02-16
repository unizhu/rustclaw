#!/usr/bin/env bash
# RustClaw One-Click Install Script for macOS/Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/unizhu/rustclaw/main/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

echo -e "${GREEN}🚀 RustClaw Installer${NC}"
echo "Detected OS: $OS"
echo "Detected Architecture: $ARCH"

# Determine the target triple
case "$OS" in
    Darwin)
        case "$ARCH" in
            x86_64|amd64)
                TARGET="x86_64-apple-darwin"
                ;;
            arm64|aarch64)
                TARGET="aarch64-apple-darwin"
                ;;
            *)
                echo -e "${RED}✗ Unsupported architecture: $ARCH${NC}"
                exit 1
                ;;
        esac
        ;;
    Linux)
        case "$ARCH" in
            x86_64|amd64)
                TARGET="x86_64-unknown-linux-gnu"
                ;;
            arm64|aarch64)
                TARGET="aarch64-unknown-linux-gnu"
                ;;
            *)
                echo -e "${RED}✗ Unsupported architecture: $ARCH${NC}"
                exit 1
                ;;
        esac
        ;;
    *)
        echo -e "${RED}✗ Unsupported OS: $OS${NC}"
        exit 1
        ;;
esac

echo "Target: $TARGET"

# Set installation directory
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="rustclaw-gateway"
ARCHIVE_NAME="rustclaw-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/unizhu/rustclaw/releases/latest/download/${ARCHIVE_NAME}"

# Create temporary directory
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

echo -e "\n${YELLOW}Downloading RustClaw...${NC}"
curl -fsSL -o "${TEMP_DIR}/${ARCHIVE_NAME}" "$DOWNLOAD_URL"

echo -e "${YELLOW}Extracting...${NC}"
tar -xzf "${TEMP_DIR}/${ARCHIVE_NAME}" -C "$TEMP_DIR"

# Check if we need sudo
if [ ! -w "$INSTALL_DIR" ]; then
    echo -e "${YELLOW}Installing to $INSTALL_DIR (requires sudo)...${NC}"
    SUDO="sudo"
else
    echo -e "${YELLOW}Installing to $INSTALL_DIR...${NC}"
    SUDO=""
fi

# Install the binary
$SUDO mv "${TEMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
$SUDO chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

# Verify installation
if command -v rustclaw-gateway &> /dev/null; then
    echo -e "\n${GREEN}✓ RustClaw installed successfully!${NC}"
    echo -e "  Location: ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "  1. Set your Telegram bot token:"
    echo "     export TELEGRAM_BOT_TOKEN=\"your_token_here\""
    echo ""
    echo "  2. Set your OpenAI API key:"
    echo "     export OPENAI_API_KEY=\"your_key_here\""
    echo ""
    echo "  3. Run RustClaw:"
    echo "     rustclaw-gateway"
    echo ""
    echo -e "${GREEN}🎉 Installation complete!${NC}"
else
    echo -e "\n${RED}✗ Installation failed${NC}"
    exit 1
fi

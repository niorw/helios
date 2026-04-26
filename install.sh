#!/bin/bash
set -e

# Helios Installer for macOS
# Usage: ./install.sh
#
# NOTE: If you change APP_NAME / ORG / QUALIFIER below,
#       also update src/config.rs to keep them in sync.

APP_NAME=$(grep '^name' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
APP_QUALIFIER="com"
APP_ORG="helios"
INSTALL_DIR="/usr/local/bin"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "⚡ ${APP_NAME} Installer"
echo "===================="

# Check OS
if [[ "$OSTYPE" != "darwin"* ]]; then
    echo "⚠️  Warning: This script is optimized for macOS. Detected: $OSTYPE"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "📦 Building ${APP_NAME} (release mode)..."
cd "$REPO_DIR"
cargo build --release

echo "🔧 Installing to ${INSTALL_DIR}..."
if [ ! -d "$INSTALL_DIR" ]; then
    sudo mkdir -p "$INSTALL_DIR"
fi
sudo cp "${REPO_DIR}/target/release/${APP_NAME}" "${INSTALL_DIR}/${APP_NAME}"
sudo chmod +x "${INSTALL_DIR}/${APP_NAME}"

echo "✅ ${APP_NAME} installed successfully!"
echo ""
echo "Usage:"
echo "  ${APP_NAME}              # Launch TUI"
echo "  ${APP_NAME} --help       # Show CLI help"
echo ""
echo "Data directory: ~/Library/Application Support/${APP_QUALIFIER}.${APP_ORG}.${APP_NAME}/"

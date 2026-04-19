#!/usr/bin/env bash
# ╔══════════════════════════════════════════════════════════╗
# ║  FLUXIS Installer — Linux / macOS / Termux               ║
# ║  Usage: curl -fsSL https://raw.githubusercontent.com/    ║
# ║         dqgamer75-oss/Fluxis/main/install.sh | bash      ║
# ╚══════════════════════════════════════════════════════════╝

set -e

REPO="https://github.com/dqgamer75-oss/Fluxis"
VERSION="4.0.0"
BINARY="fluxis"

# ── colours ──────────────────────────────────────────────────────────────
BOLD="\033[1m"
CYAN="\033[36m"
GREEN="\033[32m"
RED="\033[31m"
YELLOW="\033[33m"
GRAY="\033[90m"
RESET="\033[0m"

header() {
    echo ""
    echo -e "${BOLD}${CYAN}  🔥 FLUXIS v${VERSION} Installer${RESET}"
    echo -e "${GRAY}  DOP · Actors · Messaging · AI · ML · Graphics${RESET}"
    echo -e "${GRAY}  ─────────────────────────────────────────────${RESET}"
    echo ""
}

step()  { echo -e "${BOLD}${CYAN}  ▶  $1${RESET}"; }
ok()    { echo -e "${GREEN}  ✓  $1${RESET}"; }
warn()  { echo -e "${YELLOW}  ⚠  $1${RESET}"; }
error() { echo -e "${RED}${BOLD}  ✗  $1${RESET}"; exit 1; }

# ── detect environment ────────────────────────────────────────────────────
detect_env() {
    if [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]; then
        ENV="termux"
        INSTALL_DIR="$HOME/.local/bin"
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        ENV="macos"
        INSTALL_DIR="/usr/local/bin"
    else
        ENV="linux"
        INSTALL_DIR="/usr/local/bin"
        # fallback to user bin if no sudo
        if [ ! -w "$INSTALL_DIR" ] 2>/dev/null; then
            INSTALL_DIR="$HOME/.local/bin"
        fi
    fi
    mkdir -p "$INSTALL_DIR"
}

# ── check for Rust ────────────────────────────────────────────────────────
check_rust() {
    if command -v cargo &>/dev/null; then
        RUST_VER=$(rustc --version | awk '{print $2}')
        ok "Rust found: $RUST_VER"
        return 0
    fi
    warn "Rust not found — installing..."
    if [ "$ENV" = "termux" ]; then
        pkg install rust -y || error "Failed to install Rust via pkg"
    else
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
        source "$HOME/.cargo/env" 2>/dev/null || true
        export PATH="$HOME/.cargo/bin:$PATH"
    fi
    ok "Rust installed"
}

# ── clone and build ───────────────────────────────────────────────────────
build() {
    TMP=$(mktemp -d)
    step "Cloning FLUXIS..."

    if command -v git &>/dev/null; then
        git clone --depth 1 "$REPO" "$TMP/fluxis" 2>&1 | tail -1
    else
        warn "git not found — installing..."
        if [ "$ENV" = "termux" ]; then
            pkg install git -y
        else
            error "Please install git and re-run this script"
        fi
        git clone --depth 1 "$REPO" "$TMP/fluxis" 2>&1 | tail -1
    fi

    ok "Cloned"
    step "Building (this takes ~1 minute on first run)..."
    cd "$TMP/fluxis"
    cargo build --release 2>&1 | grep -E "Compiling|Finished|error" || true
    ok "Build complete"

    step "Installing to $INSTALL_DIR..."
    cp "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
    chmod +x "$INSTALL_DIR/$BINARY"
    ok "Installed to $INSTALL_DIR/$BINARY"

    cd /
    rm -rf "$TMP"
}

# ── PATH check ────────────────────────────────────────────────────────────
check_path() {
    if ! command -v fluxis &>/dev/null; then
        echo ""
        warn "Add $INSTALL_DIR to your PATH to use fluxis from anywhere:"
        echo ""
        echo -e "  ${CYAN}echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc${RESET}"
        echo -e "  ${CYAN}source ~/.bashrc${RESET}"
        if [ "$ENV" = "termux" ]; then
            echo ""
            echo -e "  ${GRAY}Or for Termux zsh:${RESET}"
            echo -e "  ${CYAN}echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc${RESET}"
        fi
        echo ""
    fi
}

# ── create a hello world to verify ───────────────────────────────────────
verify() {
    HELLO=$(mktemp /tmp/hello_XXXXXX.fx)
    cat > "$HELLO" << 'FX'
start {
    out("Hello from FLUXIS!")..
    out("Installation successful.")..
}
FX
    RESULT=$("$INSTALL_DIR/$BINARY" "$HELLO" 2>&1 || true)
    rm -f "$HELLO"
    if echo "$RESULT" | grep -q "Hello from FLUXIS"; then
        ok "Verified — FLUXIS is working"
    else
        warn "Could not auto-verify, but binary is installed"
    fi
}

# ── done ──────────────────────────────────────────────────────────────────
done_msg() {
    echo ""
    echo -e "${BOLD}${GREEN}  ✓ FLUXIS v${VERSION} installed!${RESET}"
    echo ""
    echo -e "  ${BOLD}Quick start:${RESET}"
    echo -e "  ${CYAN}fluxis${RESET}               — REPL"
    echo -e "  ${CYAN}fluxis hello.fx${RESET}       — run a file"
    echo -e "  ${CYAN}fluxis --help${RESET}          — all options"
    echo ""
    echo -e "  ${GRAY}Docs: https://fluxislang.netlify.app${RESET}"
    echo ""
}

# ── run ───────────────────────────────────────────────────────────────────
header
detect_env
step "Environment: $ENV"
check_rust
build
check_path
verify
done_msg

#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/programmersd21/mew.git"
INSTALL_DIR="${MEW_INSTALL_DIR:-$HOME/.local/share/mew}"
BIN_DIR="${MEW_BIN_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/mew"
THEME_FILE="$CONFIG_DIR/theme.toml"

log() {
    printf 'mew: %s\n' "$1"
}

if ! command -v cargo >/dev/null 2>&1; then
    log "cargo not found. installing rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    . "$HOME/.cargo/env"
fi

if [ -f "Cargo.toml" ] && [ -f "src/main.rs" ]; then
    MEW_SOURCE_DIR="$(pwd)"
else
    printf '%s\n' \
        "How would you like to install mew?" \
        "" \
        "1) cargo install mew-cli" \
        "2) build from GitHub source" \
        ""

    read -r -p "Choose [1/2]: " choice

    case "$choice" in
        1)
            log "installing mew-cli from crates.io..."
            cargo install mew-cli
            # provision the theme too: a crates.io install has no installer
            # hook, so this is the only chance the user gets one
            MEW_BIN="${CARGO_HOME:-$HOME/.cargo}/bin/mew"
            if [ ! -x "$MEW_BIN" ]; then
                MEW_BIN="$(command -v mew || true)"
            fi
            if [ ! -f "$THEME_FILE" ] && [ -n "$MEW_BIN" ]; then
                log "provisioning default theme..."
                mkdir -p "$CONFIG_DIR"
                "$MEW_BIN" theme > "$THEME_FILE" || log "note: could not write default theme to $THEME_FILE"
            fi
            exit 0
            ;;
        2)
            if ! command -v git >/dev/null 2>&1; then
                log "git not found. install git and run this installer again."
                exit 1
            fi

            log "cloning mew..."
            rm -rf "$INSTALL_DIR"
            git clone --depth 1 "$REPO_URL" "$INSTALL_DIR"
            MEW_SOURCE_DIR="$INSTALL_DIR"
            ;;
        *)
            log "invalid choice."
            exit 1
            ;;
    esac
fi

cd "$MEW_SOURCE_DIR"

log "compiling release binary..."
cargo build --release --locked

echo "mew: stripping binary..."
strip target/release/mew 2>/dev/null || true

OS_KIND="$(uname -s 2>/dev/null || echo Linux)"
INSTALL_DIR="/usr/local/bin"
if [ "$OS_KIND" = "Darwin" ] && [ "$(uname -m 2>/dev/null)" = "arm64" ] && [ -d "/opt/homebrew/bin" ]; then
    INSTALL_DIR="/opt/homebrew/bin"
fi
if [ ! -d "$INSTALL_DIR" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

echo "mew: installing binary to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    install -m 755 target/release/mew "$INSTALL_DIR/mew"
else
    sudo install -m 755 target/release/mew "$INSTALL_DIR/mew"
fi
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) echo "mew: note: $INSTALL_DIR is not on PATH; add it to use 'mew' directly." ;;
esac

mkdir -p "$BIN_DIR"

log "installing binary..."
install -m 755 target/release/mew "$BIN_DIR/mew"

if [ ! -f "$THEME_FILE" ]; then
    log "provisioning default theme..."
    mkdir -p "$CONFIG_DIR"
    # the annotated template is the single source of truth — never a stale copy
    "$MEW_SOURCE_DIR/target/release/mew" theme > "$THEME_FILE"
fi
HYPR_CONF="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.conf"

if [ -f "$HYPR_CONF" ] && ! grep -q 'bind = \$mainMod, M, exec' "$HYPR_CONF" 2>/dev/null; then
    log "hyprland config detected."
    printf '%s\n' '     bind = $mainMod, M, exec, alacritty --class mew -e mew'
fi

log "installation complete."
log "run 'mew hook <shell>' to integrate with your prompt."

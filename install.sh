#!/usr/bin/env bash
# htmltoapk installer: builds the release binary and installs it locally.
set -Eeuo pipefail

APP_NAME="htmltoapk"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="${BIN_DIR:-$PREFIX/bin}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

bold=""; dim=""; red=""; green=""; yellow=""; reset=""
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
	bold=$'\033[1m'; dim=$'\033[2m'; red=$'\033[31m'
	green=$'\033[32m'; yellow=$'\033[33m'; reset=$'\033[0m'
fi

info()  { printf '%s==>%s %s\n' "$bold" "$reset" "$1"; }
note()  { printf '    %s%s%s\n' "$dim" "$1" "$reset"; }
warn()  { printf '%s warn%s %s\n' "$yellow" "$reset" "$1" >&2; }
fail()  { printf '%serror%s %s\n' "$red" "$reset" "$1" >&2; exit 1; }
done_() { printf '%s  ok%s  %s\n' "$green" "$reset" "$1"; }

usage() {
	cat <<EOF
${bold}htmltoapk installer${reset}

Usage: ./install.sh [options]

Options:
  --prefix <dir>   Install prefix (default: \$HOME/.local)
  --bin-dir <dir>  Binary directory (default: <prefix>/bin)
  --debug          Install a debug build instead of release
  --no-setup       Skip the interactive \`htmltoapk setup\`
  -h, --help       Show this help

Environment: PREFIX, BIN_DIR, CARGO
EOF
}

PROFILE="release"
RUN_SETUP=1
while [ $# -gt 0 ]; do
	case "$1" in
		--prefix) [ $# -ge 2 ] || fail "--prefix needs a directory"; PREFIX="$2"; BIN_DIR="$PREFIX/bin"; shift 2 ;;
		--bin-dir) [ $# -ge 2 ] || fail "--bin-dir needs a directory"; BIN_DIR="$2"; shift 2 ;;
		--debug) PROFILE="debug"; shift ;;
		--no-setup) RUN_SETUP=0; shift ;;
		-h|--help) usage; exit 0 ;;
		*) usage; fail "unknown option: $1" ;;
	esac
done

[ "$(uname -s)" = "Linux" ] || warn "htmltoapk targets Linux; $(uname -s) is untested."

CARGO="${CARGO:-cargo}"
if ! command -v "$CARGO" >/dev/null 2>&1; then
	fail "cargo was not found.
    Install the Rust toolchain first:
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

cd "$SCRIPT_DIR"
[ -f Cargo.toml ] || fail "Cargo.toml not found in $SCRIPT_DIR"

info "Building $APP_NAME ($PROFILE)"
if [ "$PROFILE" = "release" ]; then
	"$CARGO" build --release --locked 2>/dev/null || "$CARGO" build --release
else
	"$CARGO" build
fi

BINARY="target/$PROFILE/$APP_NAME"
[ -x "$BINARY" ] || fail "build finished but $BINARY is missing"

info "Installing to $BIN_DIR"
mkdir -p "$BIN_DIR"
install -m 0755 "$BINARY" "$BIN_DIR/$APP_NAME"
done_ "$BIN_DIR/$APP_NAME"

case ":$PATH:" in
	*":$BIN_DIR:"*) ;;
	*)
		warn "$BIN_DIR is not in your PATH."
		note "Add this line to ~/.bashrc or ~/.zshrc:"
		note "  export PATH=\"$BIN_DIR:\$PATH\""
		;;
esac

if [ "$RUN_SETUP" -eq 1 ] && [ -t 0 ]; then
	info "Running first-time setup"
	"$BIN_DIR/$APP_NAME" setup || warn "setup did not finish; run '$APP_NAME setup' later"
else
	note "Run '$APP_NAME setup' to create the configuration file."
fi

printf '\n%sInstalled.%s Start the TUI with %s%s%s, or check the environment with %s%s doctor%s.\n\n' \
	"$green" "$reset" "$bold" "$APP_NAME" "$reset" "$bold" "$APP_NAME" "$reset"

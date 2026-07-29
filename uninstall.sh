#!/usr/bin/env bash
# htmltoapk uninstaller: removes the binary and (optionally) user data.
set -Eeuo pipefail

APP_NAME="htmltoapk"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="${BIN_DIR:-$PREFIX/bin}"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_NAME"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/$APP_NAME"

info() { printf '==> %s\n' "$1"; }
note() { printf '    %s\n' "$1"; }
warn() { printf 'warn: %s\n' "$1" >&2; }
fail() { printf 'error: %s\n' "$1" >&2; exit 1; }
okay() { printf ' ok: %s\n' "$1"; }

usage() {
	cat <<EOF
htmltoapk uninstaller

Usage: ./uninstall.sh [options]

Options:
  --prefix <dir>   Install prefix (default: \$HOME/.local)
  --bin-dir <dir>  Binary directory (default: <prefix>/bin)
  --purge          Also delete configuration, workspaces and logs
  --yes, -y        Do not ask for confirmation
  -h, --help       Show this help
EOF
}

PURGE=0
ASSUME_YES=0
while [ $# -gt 0 ]; do
	case "$1" in
		--prefix)
			[ $# -ge 2 ] || fail "--prefix needs a directory"
			PREFIX="$2"
			BIN_DIR="$PREFIX/bin"
			shift 2
			;;
		--bin-dir)
			[ $# -ge 2 ] || fail "--bin-dir needs a directory"
			BIN_DIR="$2"
			shift 2
			;;
		--purge)
			PURGE=1
			shift
			;;
		--yes|-y)
			ASSUME_YES=1
			shift
			;;
		-h|--help)
			usage
			exit 0
			;;
		*)
			usage
			fail "unknown option: $1"
			;;
	esac
done

confirm() {
	if [ "$ASSUME_YES" -eq 1 ]; then
		return 0
	fi
	if [ ! -t 0 ]; then
		return 1
	fi
	printf '%s [y/N] ' "$1"
	read -r answer
	case "$answer" in
		[yY]|[yY][eE][sS]) return 0 ;;
		*) return 1 ;;
	esac
}

REMOVED=0

if [ -e "$BIN_DIR/$APP_NAME" ]; then
	info "Removing $BIN_DIR/$APP_NAME"
	rm -f "$BIN_DIR/$APP_NAME"
	okay "binary removed"
	REMOVED=1
else
	warn "no binary at $BIN_DIR/$APP_NAME"
	if command -v "$APP_NAME" >/dev/null 2>&1; then
		note "another copy is still on PATH: $(command -v "$APP_NAME")"
	fi
fi

if [ "$PURGE" -eq 1 ]; then
	for dir in "$CONFIG_DIR" "$DATA_DIR"; do
		if [ ! -d "$dir" ]; then
			continue
		fi
		size="$(du -sh "$dir" 2>/dev/null | cut -f1 || true)"
		if [ -z "$size" ]; then
			size="unknown size"
		fi
		if confirm "Delete $dir ($size)?"; then
			rm -rf "$dir"
			okay "$dir removed"
			REMOVED=1
		else
			note "kept $dir"
		fi
	done
else
	if [ -d "$CONFIG_DIR" ]; then
		note "configuration kept in $CONFIG_DIR (use --purge to delete)"
	fi
	if [ -d "$DATA_DIR" ]; then
		note "workspaces and logs kept in $DATA_DIR (use --purge to delete)"
	fi
fi

if [ "$REMOVED" -eq 1 ]; then
	printf '\nUninstalled.\n\n'
else
	printf '\nNothing was removed.\n\n'
fi

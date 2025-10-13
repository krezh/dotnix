#!/usr/bin/env bash
# Extract package source information (owner/repo) from package definition

set -euo pipefail

PACKAGE="${1:-}"

if [ -z "$PACKAGE" ]; then
  echo "Usage: $0 <package-name>"
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

PKG_FILE=""

if [ -f "$ROOT_DIR/pkgs/bin/$PACKAGE/default.nix" ]; then
  PKG_FILE="$ROOT_DIR/pkgs/bin/$PACKAGE/default.nix"
elif [ -f "$ROOT_DIR/pkgs/scripts/$PACKAGE/default.nix" ]; then
  PKG_FILE="$ROOT_DIR/pkgs/scripts/$PACKAGE/default.nix"
else
  PKG_DIR=$(find "$ROOT_DIR/pkgs" -type d -name "$PACKAGE" 2>/dev/null | head -n1)
  if [ -n "$PKG_DIR" ] && [ -f "$PKG_DIR/default.nix" ]; then
    PKG_FILE="$PKG_DIR/default.nix"
  fi
fi

if [ -n "$PKG_FILE" ] && [ -f "$PKG_FILE" ]; then
  OWNER=$(grep -oP 'owner\s*=\s*"\K[^"]+' "$PKG_FILE" || echo "unknown")
  REPO=$(grep -oP 'repo\s*=\s*"\K[^"]+' "$PKG_FILE" || echo "$PACKAGE")

  echo "owner=$OWNER"
  echo "repo=$REPO"
else
  echo "owner=unknown"
  echo "repo=$PACKAGE"
fi

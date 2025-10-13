#!/usr/bin/env bash
# Script to list packages from flake that use remote fetchers
# Uses flake metadata as source of truth, then checks actual files in repo

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$ROOT_DIR"

# Get all packages from the flake for the current system
SYSTEM="${1:-$(nix eval --impure --raw --expr 'builtins.currentSystem' 2>/dev/null || echo "x86_64-linux")}"

# Get package list from flake
PACKAGES=$(nix flake show --json 2>/dev/null | jq -r ".packages.\"$SYSTEM\" | keys[]" | grep -v "^gc-keep$" || true)

if [ -z "$PACKAGES" ]; then
  echo "[]"
  exit 0
fi

REMOTE_PACKAGES=()

for pkg in $PACKAGES; do
  # Intelligently search for package file in pkgs/ directory structure
  # This handles both nested (pkgs/bin/foo) and flat (pkgs/foo) structures
  PKG_FILE=""

  # Method 1: Search for the package directory anywhere under pkgs/
  PKG_DIR=$(find "$ROOT_DIR/pkgs" -type d -name "$pkg" 2>/dev/null | head -n1)

  if [ -n "$PKG_DIR" ] && [ -f "$PKG_DIR/default.nix" ]; then
    PKG_FILE="$PKG_DIR/default.nix"
  elif [ -f "$ROOT_DIR/pkgs/$pkg/default.nix" ]; then
    # Method 2: Flat structure at pkgs/package-name/default.nix
    PKG_FILE="$ROOT_DIR/pkgs/$pkg/default.nix"
  elif [ -f "$ROOT_DIR/pkgs/$pkg.nix" ]; then
    # Method 3: Single file at pkgs/package-name.nix
    PKG_FILE="$ROOT_DIR/pkgs/$pkg.nix"
  fi

  # If we found a package file, check if it uses remote fetchers
  if [ -n "$PKG_FILE" ] && [ -f "$PKG_FILE" ]; then
    if grep -Eq 'fetchFromGitHub|fetchFromGitLab|fetchFromGitea|fetchFromSourcehut|fetchurl|fetchgit' "$PKG_FILE"; then
      REMOTE_PACKAGES+=("$pkg")
    fi
  fi
done

# Output as JSON array for GitHub Actions matrix
if [ ${#REMOTE_PACKAGES[@]} -eq 0 ]; then
  echo "[]"
else
  printf '%s\n' "${REMOTE_PACKAGES[@]}" | jq -R -s -c 'split("\n") | map(select(length > 0))'
fi

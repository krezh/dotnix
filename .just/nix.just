[private]
default:
    @just --list nix

ROOT_DIR := "$(git rev-parse --show-toplevel)"
HOST := "$(hostname -s)"

[private]
deps:
    #!/usr/bin/env bash
    set -euo pipefail
    deps=("nix" "nix-update" "nixos-rebuild" "nvd")
    for cmd in "${deps[@]}"; do
      if ! command -v "$cmd" &> /dev/null; then
        just log error "$cmd is not installed."
        exit 1
      fi
    done

# Update nix packages
update-packages: deps
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION_REGEX='^v?([0-9]+)\.([0-9]+)\.([0-9]+)(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$'
    PKGS=$(ls -1 {{ ROOT_DIR }}/pkgs/bin -I default.nix)
    cd {{ ROOT_DIR }}
    # Count packages for progress tracking
    TOTAL_PKGS=$(echo "$PKGS" | wc -l)
    CURRENT=0
    for pkg in $PKGS; do
      CURRENT=$((CURRENT + 1))
      PKG_NIX="{{ ROOT_DIR }}/pkgs/bin/${pkg}/default.nix"
      # Check for remote fetchers
      if grep -Eq 'fetchFromGitHub|fetchFromGitLab|fetchurl' "$PKG_NIX"; then
        just log info "[$CURRENT/$TOTAL_PKGS] Updating remote package: $pkg"
        # Get current version before update
        OLD_VERSION=$(nix eval ".#$pkg.version" --raw 2>/dev/null || echo "unknown")
        # Capture output and only show on error
        if ! nix-update "$pkg" \
          --quiet --flake \
          --commit --format \
          --generate-lockfile \
          --version-regex "$VERSION_REGEX" \
          >/dev/null 2>&1; then
          just log error "Failed to update $pkg"
        else
          # Get new version after update
          NEW_VERSION=$(nix eval ".#$pkg.version" --raw 2>/dev/null || echo "unknown")

          if [ "$OLD_VERSION" != "$NEW_VERSION" ]; then
            just log info "Updated $pkg: $OLD_VERSION → $NEW_VERSION"
          else
            just log info "Updated $pkg (no version change)"
          fi
        fi
      else
        just log info "[$CURRENT/$TOTAL_PKGS] Skipping local package: $pkg"
      fi
    done
    just log info "Package update completed!"

# Build nix configuration
[no-cd]
build: deps
    #!/usr/bin/env bash
    set -euo pipefail
    git pull
    nixos-rebuild build --flake "{{ ROOT_DIR }}/#{{ HOST }}"
    nvd diff /run/current-system result

# Build and apply nix configuration
[no-cd]
apply: build
    #!/usr/bin/env bash
    set -euo pipefail
    just confirm "Do you want to continue applying this configuration?"
    sudo nixos-rebuild switch --flake "{{ ROOT_DIR }}/#{{ HOST }}"

# Build a specific pull request from GitHub
[no-cd]
fast-build-pr pr:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run nixpkgs#nix-fast-build -- \
      --skip-cached --flake "github:krezh/nix-config/pull/{{ pr }}/head#top"

# Build the top-level flake
[no-cd]
fast-build:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run nixpkgs#nix-fast-build -- --skip-cached --flake ".#top"

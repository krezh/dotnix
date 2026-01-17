[private]
default:
    @just --list nix

ROOT_DIR := "$(git rev-parse --show-toplevel)"
HOST := "$(hostname -s)"

[private]
deps:
    #!/usr/bin/env bash
    set -euo pipefail
    deps=("nix" "nix-update" "nixos-rebuild" "dix")
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
    JOBS=8
    cd {{ ROOT_DIR }}
    PKGS=$(ls -1 {{ ROOT_DIR }}/pkgs/bin -I default.nix)
    TOTAL_PKGS=$(echo "$PKGS" | wc -l)
    REMOTE_PKGS=()
    CURRENT=0

    for pkg in $PKGS; do
      CURRENT=$((CURRENT + 1))
      PKG_NIX="{{ ROOT_DIR }}/pkgs/bin/${pkg}/default.nix"
      if grep -Eq 'fetchFromGitHub|fetchFromGitLab|fetchurl' "$PKG_NIX"; then
        REMOTE_PKGS+=("$pkg")
        just log info "[$CURRENT/$TOTAL_PKGS] Queued: $pkg"
      else
        just log info "[$CURRENT/$TOTAL_PKGS] Skipping local package: $pkg"
      fi
    done

    if [ ${#REMOTE_PKGS[@]} -eq 0 ]; then
      just log info "No remote packages to update."
      exit 0
    fi

    just log info "Updating ${#REMOTE_PKGS[@]} remote packages with $JOBS parallel jobs..."

    update_single_pkg() {
      local pkg=$1
      just log info "Updating: $pkg"
      local OLD_VERSION=$(nix eval ".#$pkg.version" --raw 2>/dev/null || echo "unknown")
      
      local UPDATE_FLAGS="--quiet --flake --commit --format --use-update-script --generate-lockfile --version-regex \"$VERSION_REGEX\""
      
      if ! eval "nix-update \"$pkg\" $UPDATE_FLAGS" >/dev/null 2>&1; then
        just log error "Failed: $pkg"
        return 1
      else
        local NEW_VERSION=$(nix eval ".#$pkg.version" --raw 2>/dev/null || echo "unknown")
        if [ "$OLD_VERSION" != "$NEW_VERSION" ]; then
          just log info "Finished: $pkg ($OLD_VERSION → $NEW_VERSION)"
        else
          just log info "Finished: $pkg (no version change)"
        fi
      fi
    }
    export -f update_single_pkg

    printf '%s\n' "${REMOTE_PKGS[@]}" | xargs -I {} -P "$JOBS" bash -c 'update_single_pkg "$@"' _ "{}"

    just log info "Package update completed!"

# Build nix configuration
[no-cd]
build: deps
    #!/usr/bin/env bash
    set -euo pipefail
    git pull
    nixos-rebuild build --flake "{{ ROOT_DIR }}/#{{ HOST }}"
    dix diff /run/current-system result

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
      --skip-cached --flake "github:krezh/nix-config/pull/{{ pr }}/head#hosts"

# Build all hosts
[no-cd]
fast-build:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run "nixpkgs#nix-fast-build" -- --skip-cached --flake ".#hosts"

# Initialize a new nix package
[no-cd]
init package:
    #!/usr/bin/env bash
    set -euo pipefail
    nix-init pkgs/bin/{{ package }}/default.nix

roots:
  #!/usr/bin/env bash
  nix-store --gc --print-roots

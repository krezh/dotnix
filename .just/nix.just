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
    PKGS=$(ls -1 {{ ROOT_DIR }}/pkgs/bin -I default.nix)
    cd {{ ROOT_DIR }}
    for pkg in $PKGS; do
      just log info "Updating package: $pkg"
      nix-update --flake --commit --format --generate-lockfile "$pkg"
    done

# Build nix configuration
build: deps
    #!/usr/bin/env bash
    set -euo pipefail
    nixos-rebuild build --flake "{{ ROOT_DIR }}/#{{ HOST }}"
    nvd diff /run/current-system result

# Build and apply nix configuration
apply: build
    #!/usr/bin/env bash
    set -euo pipefail
    just prompt-to-continue "Do you want to continue applying this configuration?"
    sudo nixos-rebuild switch --flake "{{ ROOT_DIR }}/#{{ HOST }}"

# Build a specific pull request from GitHub
[no-cd]
fast-build-pr pr:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run nixpkgs#nix-fast-build -- --skip-cached --flake "github:krezh/nix-config/pull/{{ pr }}/head#top"

# Build a specific pull request from GitHub
[no-cd]
fast-build:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run nixpkgs#nix-fast-build -- --skip-cached --flake ".#top"


set lazy

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
update-packages pkg="": deps
  #!/usr/bin/env bash
  set -euo pipefail
  JOBS=8
  cd {{ ROOT_DIR }}
  REMOTE_PKGS=()

  if [ -n "{{ pkg }}" ]; then
    REMOTE_PKGS+=("{{ pkg }}")
    just log info "Targeted update: {{ pkg }}"
  else
    PKGS=$(ls -1 {{ ROOT_DIR }}/pkgs/bin -I default.nix)
    TOTAL_PKGS=$(echo "$PKGS" | wc -l)
    CURRENT=0

    for p in $PKGS; do
      CURRENT=$((CURRENT + 1))
      PKG_NIX="{{ ROOT_DIR }}/pkgs/bin/${p}/default.nix"
      # Queue only if it has a remote source AND version is semver (e.g. 1.2.3 or v1.2)
      if grep -Eq 'src = (pkgs\.)?fetch' "$PKG_NIX" && \
        grep -Eq 'version = "v?[0-9]+\.[0-9]+' "$PKG_NIX"; then
        REMOTE_PKGS+=("$p")
        just log info "[$CURRENT/$TOTAL_PKGS] Queued: $p"
      else
        just log info "[$CURRENT/$TOTAL_PKGS] Skipping: $p"
      fi
    done
  fi

  if [ ${#REMOTE_PKGS[@]} -eq 0 ]; then
    just log info "No remote packages to update."
    exit 0
  fi

  just log info "Updating ${#REMOTE_PKGS[@]} package(s) with $JOBS parallel jobs..."

  update_single_pkg() {
    local p=$1
    export TERM=dumb
    just log info "Updating: $p"

    # Get current version for comparison
    local OLD_VERSION=$(nix eval ".#$p.version" --raw 2>/dev/null || echo "unknown")

    # Run nix-update
    set +e
    nix-update "$p" --flake --generate-lockfile >/dev/null 2>&1
    local EXIT_CODE=$?
    set -e

    if [ $EXIT_CODE -ne 0 ]; then
      just log error "Failed: $p (exit $EXIT_CODE)"
      return 1
    fi

    # Check if version changed
    local NEW_VERSION=$(nix eval ".#$p.version" --raw 2>/dev/null || echo "unknown")

    if [ "$OLD_VERSION" != "$NEW_VERSION" ]; then
      just log info "Finished: $p ($OLD_VERSION → $NEW_VERSION)"
    else
      just log info "Finished: $p (no version change)"
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
  nixos-rebuild build --flake "{{ ROOT_DIR }}/#{{ HOST }}"
  dix /run/current-system $(pwd)/result

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
    --skip-cached --option fallback true --flake "github:krezh/dotnix/pull/{{ pr }}/head#hosts"

# Build all hosts
[no-cd]
fast-build args:
  #!/usr/bin/env bash
  set -euo pipefail
  nix run nixpkgs#nix-fast-build -- \
    --skip-cached --option fallback true --flake ".#hosts" {{ args }}

# Initialize a new nix package
[no-cd]
init package:
  #!/usr/bin/env bash
  set -euo pipefail
  nix-init pkgs/bin/{{ package }}/default.nix

roots:
  #!/usr/bin/env bash
  nix-store --gc --print-roots

# Update mime-types list
update-mimes:
  #!/usr/bin/env bash
  set -euo pipefail
  MIME_PATH=$(nix-build -E 'with import <nixpkgs> {}; shared-mime-info' --no-out-link)/share/mime/types
  OUTPUT="{{ ROOT_DIR }}/modules/programs/xdg-settings/_mimetypes.nix"
  echo "{" > "$OUTPUT"
  echo "  allMimes = [" >> "$OUTPUT"
  # Filter out empty lines and wrap in quotes
  grep . "$MIME_PATH" | sed 's/^/    "/' | sed 's/$/"/' >> "$OUTPUT"
  echo "  ];" >> "$OUTPUT"
  echo "}" >> "$OUTPUT"
  just log info "Mime-types updated in $OUTPUT"

# Find available .desktop files on the system
find-desktops pattern="":
  #!/usr/bin/env bash
  set -euo pipefail
  find /run/current-system/sw/share/applications /etc/profiles/per-user/$USER/share/applications $HOME/.local/share/applications -name "*{{ pattern }}*.desktop" -printf "%f\n" 2>/dev/null | sort -u

# List .desktop files provided by a specific package
pkg-desktops pkg:
  #!/usr/bin/env bash
  set -euo pipefail
  OUT_PATH=$(nix-build -E "with import <nixpkgs> {}; {{ pkg }}" --no-out-link)
  find "$OUT_PATH" -name "*.desktop" -printf "%f\n"

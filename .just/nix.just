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
      --skip-cached --flake "github:krezh/nix-config/pull/{{ pr }}/head#hosts" --attic-cache krezh

# Build all hosts
[no-cd]
fast-build:
    #!/usr/bin/env bash
    set -euo pipefail
    nix run nixpkgs#nix-fast-build -- --skip-cached --flake ".#hosts" --attic-cache krezh

# Initialize a new nix package
[no-cd]
init package:
    #!/usr/bin/env bash
    set -euo pipefail
    nix-init pkgs/bin/{{ package }}/default.nix

roots:
  #!/usr/bin/env bash
  nix-store --gc --print-roots

# Test the KubeVirt image with QEMU
[no-cd]
test-kubevirt:
    #!/usr/bin/env bash
    set -euo pipefail
    just log info "Building KubeVirt image..."
    nix build "{{ ROOT_DIR }}#images.kubevirt"
    IMAGE=$(ls -1 result/*.qcow2)
    TMPDIR=$(mktemp -d)
    trap "rm -rf $TMPDIR" EXIT
    echo "instance-id: test" > $TMPDIR/meta-data
    echo "local-hostname: kubevirt" >> $TMPDIR/meta-data
    echo "#cloud-config" > $TMPDIR/user-data
    $(nix-build '<nixpkgs>' -A cdrkit --no-out-link)/bin/genisoimage \
      -output $TMPDIR/seed.iso -volid cidata -joliet -rock \
      $TMPDIR/user-data $TMPDIR/meta-data 2>/dev/null
    just log info "Starting QEMU (SSH on port 2222, Ctrl+A then X to exit)"
    $(nix-build '<nixpkgs>' -A qemu --no-out-link)/bin/qemu-system-x86_64 \
      -m 2048 -enable-kvm \
      -drive file=$IMAGE,format=qcow2,snapshot=on \
      -drive file=$TMPDIR/seed.iso,format=raw \
      -netdev user,id=net0,hostfwd=tcp::2222-:22 \
      -device virtio-net-pci,netdev=net0 -nographic

# SSH into the running KubeVirt test VM
[no-cd]
ssh-kubevirt:
    #!/usr/bin/env bash
    set -euo pipefail
    just log info "Connecting to SSH (waiting up to 30s)..."
    for i in {1..30}; do
      if ssh -p 2222 -o ConnectTimeout=2 -o StrictHostKeyChecking=no root@localhost true 2>/dev/null; then
        ssh -p 2222 root@localhost
        exit 0
      fi
      sleep 1
    done
    just log error "SSH connection timed out"
    exit 1
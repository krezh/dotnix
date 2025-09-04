[private]
default:
  @just --list sops

# Re-encrypt all sops files
re-encrypt:
  #!/usr/bin/env bash
  set -euo pipefail
  SECRET_FILES=$(find {{ justfile_dir() }} -type f -name '*.sops.yaml' ! -name ".sops.yaml")
  for file in $SECRET_FILES; do
    just log info "Re-encrypting $file"
    sops --rotate --in-place "$file"
  done

# Update all sops keys
update-keys:
  #!/usr/bin/env bash
  set -euo pipefail
  SECRET_FILES=$(find {{ justfile_dir() }} -type f -name '*.sops.yaml' ! -name ".sops.yaml")
  for file in $SECRET_FILES; do
    sops updatekeys "$file"
  done

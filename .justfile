[private]
default:
  @just --list

mod sops '.just/sops.just'
mod nix '.just/nix.just'

[private, positional-arguments]
log lvl msg *args:
  @gum log -t rfc3339 -s -l "{{ lvl }}" "{{ msg }}" {{ args }}

[private, positional-arguments]
prompt-to-continue msg:
  #!/usr/bin/env bash
  set -euo pipefail
  read -p "{{ msg }} [y/N] " ans
  case "$ans" in
    [Yy]*) ;;
    *) echo "Aborted."; exit 1 ;;
  esac

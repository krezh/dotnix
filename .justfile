[private]
default:
  @just --list

mod sops '.just/sops.just'
mod nix '.just/nix.just'

[private, positional-arguments]
log lvl msg *args:
    @gum log -t rfc3339 -s -l "{{ lvl }}" "{{ msg }}" {{ args }}

[private, positional-arguments]
confirm msg:
    gum confirm "{{ msg }}"

[private, positional-arguments]
choose msg *options:
    #!/usr/bin/env bash
    set -euo pipefail
    SELECTED="$(gum choose --header "{{ msg }}" {{options}})"
    echo "$SELECTED"

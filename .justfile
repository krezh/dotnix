set quiet
set lazy
set no-exit-message
set positional-arguments

[private]
default:
    @just --list

mod sops '.just/sops.just'
mod nix '.just/nix.just'

[private]
log lvl msg *args:
    @gum log -t rfc3339 -s -l "{{ lvl }}" "{{ msg }}" {{ args }}

[private]
confirm msg:
    gum confirm "{{ msg }}"

[private]
choose msg *options:
    #!/usr/bin/env bash
    set -euo pipefail
    SELECTED="$(gum choose --header "{{ msg }}" {{ options }})"
    echo "$SELECTED"

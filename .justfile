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

ensure-env:
  #!/usr/bin/env bash
  set -euo pipefail
  if [[ -z "${IN_NIX_SHELL:-}" ]]; then
    echo "🚀 Entering Nix shell with python3 and inotify-tools..."
    exec nix shell nixpkgs#python3 nixpkgs#inotify-tools
  else
    echo "✅ Already inside Nix shell."
  fi


test-pkg-diff *args:
  #!/usr/bin/env bash
  set -euo pipefail

  build_targets=(thor odin)
  outputs=()
  pkg_lists=()

  echo "🔨 Building systems in parallel (only if missing)..."
  for target in "${build_targets[@]}"; do
    out_link="result-$target"
    pkg_json="pkg-list-$target.json"
    outputs+=("$out_link")
    pkg_lists+=("$pkg_json")

    (
      if [[ ! -e "$out_link" ]]; then
        echo "→ Building .#top.$target"
        nix build ".#top.$target" -o "$out_link" --keep-going --print-out-paths >/dev/null
      else
        echo "✓ Using existing $out_link"
      fi

      echo "📦 Listing packages for $target..."
      python3 .github/pkg-tool.py list "$out_link" "$pkg_json"
    ) &
  done

  # Wait for *all* parallel builds + list generations to complete
  wait

  echo "🧩 Diffing package lists..."
  if [[ -f "${pkg_lists[0]}" && -f "${pkg_lists[1]}" ]]; then
      diff_output=$(python3 .github/pkg-tool.py diff "${pkg_lists[0]}" "${pkg_lists[1]}" {{ args }} || true)
    if [[ -z "$diff_output" ]]; then
      echo "✅ No differences found."
    else
      echo "$diff_output" | tee >(wl-copy)
      echo "✅ Diff copied to clipboard."
    fi
  else
    echo "❌ Missing package list files — something failed upstream."
    exit 1
  fi


watch-test-pkg-diff:
  #!/usr/bin/env bash
  set -euo pipefail

  # Enter Nix shell if not already inside
  if [[ -z "${IN_NIX_SHELL:-}" ]]; then
    echo "🚀 Entering Nix shell with python3 and inotify-tools..."
    exec nix shell nixpkgs#python3 nixpkgs#inotify-tools --command just watch-test-pkg-diff
  fi

  echo "👀 Watching for changes in .nix, .py, and .just files..."

  inotifywait -m -r \
    --event modify,create,delete,move \
    --include '.*\.(nix|py|just)$' \
    --format '%w%f' . \
  | while read -r file; do
      clear
      echo "🔄 Change detected in: $file"
      just test-pkg-diff
      echo "🔁 Watching again..."
    done

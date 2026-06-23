#!/usr/bin/env bash
set -euo pipefail

file="$1"

[ -n "${RENOVATE_TOKEN:-}" ] && export NIX_CONFIG="access-tokens = github.com=$RENOVATE_TOKEN"

# Redact IPs from nix stderr — GitHub rate limit errors include the requester's
# external IP in their response body, which Renovate posts verbatim in PR comments.
nix() {
    local stderr_tmp rc=0
    stderr_tmp=$(mktemp)
    command nix "$@" 2>"$stderr_tmp" || rc=$?
    sed -E 's/\b([0-9]{1,3}\.){3}[0-9]{1,3}\b/<redacted-ip>/g' "$stderr_tmp" >&2
    rm -f "$stderr_tmp"
    return $rc
}

# Check if this is a crane package
if grep -q "craneLib\.buildPackage" "$file"; then
  echo "Updating crane package: $file"

  owner=$(grep -oP 'owner = "\K[^"]*' "$file")
  repo=$(grep -oP 'repo = "\K[^"]*' "$file")
  version=$(grep -P '^\s*version = ' "$file" | cut -d\" -f2)
  ref=$(grep -oP '(tag|rev) = "\K[^"]*' "$file" | head -1 | sed "s/\${version}/$version/g")

  echo "  fetchFromGitHub: $owner/$repo @ $ref"

  old_hash=$(grep -oP 'hash = "\K[^"]*' "$file")
  echo "  Old hash: $old_hash"

  sri=$(nix store prefetch-file --json "https://github.com/$owner/$repo/archive/refs/tags/$ref.tar.gz" --unpack | jq -r .hash)

  echo "  New hash: $sri"

  if [ "$old_hash" = "$sri" ]; then
    echo "  ✓ Hash unchanged"
  else
    echo "  → Hash updated"
    sed -i "s|hash = \"sha256-[^\"]\+\";|hash = \"$sri\";|" "$file"

    # Verify the file still parses
    nix-instantiate --parse "$file" >/dev/null 2>&1 || {
      echo "  ✗ Error: File no longer parses correctly after update!"
      exit 1
    }
  fi
else
  # Not a crane package, use nix-update with the pinned nixpkgs from flake.lock
  # (avoids the GitHub API call that `nix run nixpkgs#...` makes to resolve nixpkgs-unstable)
  echo "Using nix-update for: $file"
  pkg=$(grep -P '^\s*pname = ' "$file" | cut -d\" -f2)
  nixpkgs_url="$(jq -r '.nodes.nixpkgs.locked | if .type == "github" then "github:\(.owner)/\(.repo)/\(.rev)" else "github:NixOS/nixpkgs/\(.rev)" end' flake.lock)"
  nix run "${nixpkgs_url}#nix-update" -- "$pkg" --flake --version=skip
fi

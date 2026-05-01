#!/usr/bin/env bash
set -euo pipefail

# Ensure nix is in PATH
export PATH="/home/ubuntu/.nix-profile/bin:${PATH:-/usr/local/bin:/usr/bin:/bin}"

file="$1"

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

  sri=$(nix --extra-experimental-features 'nix-command flakes' store prefetch-file --json "https://github.com/$owner/$repo/archive/refs/tags/$ref.tar.gz" --unpack | jq -r .hash)

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
  # Not a crane package, use nix-update
  echo "Using nix-update for: $file"
  pkg=$(grep -P '^\s*pname = ' "$file" | cut -d\" -f2)
  nix --extra-experimental-features 'nix-command flakes' run github:Mic92/nix-update -- "$pkg" --flake --version=skip
fi

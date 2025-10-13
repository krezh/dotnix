#!/usr/bin/env bash
# Script to fetch changelog information from GitHub releases
# Similar to Renovate's changelog functionality

set -euo pipefail

OWNER="${1:-}"
REPO="${2:-}"
OLD_VERSION="${3:-}"
NEW_VERSION="${4:-}"
GITHUB_TOKEN="${5:-}"

if [ -z "$OWNER" ] || [ -z "$REPO" ] || [ -z "$OLD_VERSION" ] || [ -z "$NEW_VERSION" ]; then
  echo "Usage: $0 <owner> <repo> <old_version> <new_version> [github_token]"
  exit 1
fi

# Normalize version tags (remove 'v' prefix if present for comparison)
normalize_version() {
  local version="$1"
  echo "${version#v}"
}

OLD_VER_NORMALIZED=$(normalize_version "$OLD_VERSION")
NEW_VER_NORMALIZED=$(normalize_version "$NEW_VERSION")

# Check if versions are the same
if [ "$OLD_VER_NORMALIZED" = "$NEW_VER_NORMALIZED" ]; then
  echo "No version change detected."
  exit 0
fi

# GitHub API base URL
API_BASE="https://api.github.com"
AUTH_HEADER=""
if [ -n "$GITHUB_TOKEN" ]; then
  AUTH_HEADER="Authorization: Bearer $GITHUB_TOKEN"
fi

# Function to call GitHub API with rate limit handling
github_api() {
  local endpoint="$1"
  local response
  local http_code

  if [ -n "$AUTH_HEADER" ]; then
    response=$(curl -s -w "\n%{http_code}" -H "$AUTH_HEADER" -H "Accept: application/vnd.github+json" "$API_BASE$endpoint")
  else
    response=$(curl -s -w "\n%{http_code}" -H "Accept: application/vnd.github+json" "$API_BASE$endpoint")
  fi

  http_code=$(echo "$response" | tail -n1)
  body=$(echo "$response" | sed '$d')

  # Check for rate limiting
  if [ "$http_code" = "403" ] || [ "$http_code" = "429" ]; then
    echo "⚠️ GitHub API rate limit reached. Some information may be unavailable." >&2
    return 1
  fi

  echo "$body"
}

# Function to get specific release by tag (more efficient than fetching all)
get_release_by_tag() {
  local version="$1"
  # Try with 'v' prefix first, then without
  for try_version in "v$version" "$version"; do
    local release
    release=$(github_api "/repos/$OWNER/$REPO/releases/tags/$try_version" 2>/dev/null || echo "")
    if [ -n "$release" ] && [ "$release" != "null" ] && echo "$release" | jq -e '.tag_name' >/dev/null 2>&1; then
      echo "$release"
      return 0
    fi
  done
  return 1
}

# Generate compare URL
COMPARE_URL="https://github.com/$OWNER/$REPO/compare/$OLD_VERSION...$NEW_VERSION"

# Try to get release notes for new version (single API call instead of fetching all releases)
NEW_RELEASE=$(get_release_by_tag "$NEW_VER_NORMALIZED" || echo "")

# Output markdown
cat <<EOF
### 🔗 Links

- [Compare $OLD_VERSION → $NEW_VERSION]($COMPARE_URL)
- [Repository](https://github.com/$OWNER/$REPO)
- [Releases](https://github.com/$OWNER/$REPO/releases)

EOF

if [ -n "$NEW_RELEASE" ] && [ "$NEW_RELEASE" != "null" ]; then
  RELEASE_NAME=$(echo "$NEW_RELEASE" | jq -r '.name // empty')
  RELEASE_URL=$(echo "$NEW_RELEASE" | jq -r '.html_url // empty')
  RELEASE_BODY=$(echo "$NEW_RELEASE" | jq -r '.body // empty')

  cat <<EOF
### 📝 Release Notes

#### [$RELEASE_NAME]($RELEASE_URL)

EOF

  if [ -n "$RELEASE_BODY" ] && [ "$RELEASE_BODY" != "null" ] && [ "$RELEASE_BODY" != "" ]; then
    # Limit release notes to first 50 lines to avoid huge PRs
    echo "$RELEASE_BODY" | head -n 50
    LINE_COUNT=$(echo "$RELEASE_BODY" | wc -l)
    if [ "$LINE_COUNT" -gt 50 ]; then
      echo ""
      echo "_... (truncated, see [full release notes]($RELEASE_URL) for more)_"
    fi
  else
    echo "_No release notes available._"
  fi
  echo ""
else
  echo "### 📝 Release Notes"
  echo ""
  echo "_Release notes not found. See [compare view]($COMPARE_URL) for changes._"
  echo ""
fi

# Get commits between versions
echo "### 📊 Commits"
echo ""

# Try to get commits from GitHub API
COMMITS=$(github_api "/repos/$OWNER/$REPO/compare/$OLD_VERSION...$NEW_VERSION" 2>/dev/null || echo "")

if [ -n "$COMMITS" ] && [ "$COMMITS" != "null" ]; then
  COMMIT_COUNT=$(echo "$COMMITS" | jq -r '.total_commits // 0')

  if [ "$COMMIT_COUNT" -gt 0 ]; then
    echo "**$COMMIT_COUNT commit(s)** between versions"
    echo ""

    # Show first 10 commits
    echo "$COMMITS" | jq -r '.commits[:10] | .[] | "- [`\(.sha[:7])`](\(.html_url)) \(.commit.message | split("\n")[0])"' || true

    if [ "$COMMIT_COUNT" -gt 10 ]; then
      echo ""
      echo "_... and $((COMMIT_COUNT - 10)) more commits. See [compare view]($COMPARE_URL) for full list._"
    fi
  else
    echo "_No commits found between versions._"
  fi
else
  echo "_Commit information unavailable. See [compare view]($COMPARE_URL)._"
fi

echo ""

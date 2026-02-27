#!/usr/bin/env bash
set -euo pipefail

# Kopia repository initialization script

REPO_TYPE="${1:-filesystem}"
REPO_PATH="${2:-$HOME/.local/share/kopia-repository}"
CONFIG_FILE="${3:-$HOME/.config/kopia/repository.config}"
PASSWORD_FILE="${4:-$HOME/.config/kopia/repository.password}"
REQUIRE_MOUNT="${5:-false}"

echo "Initializing Kopia repository..."
echo "Type: $REPO_TYPE"
echo "Path: $REPO_PATH"
echo "Config: $CONFIG_FILE"

if [[ "$REQUIRE_MOUNT" == "true" ]]; then
  if ! mountpoint -q "$REPO_PATH"; then
    echo "ERROR: Repository path '$REPO_PATH' is not a mount point" >&2
    exit 1
  fi
  echo "Mount point check passed: $REPO_PATH"
fi

# Create directories
mkdir -p "$(dirname "$CONFIG_FILE")"
mkdir -p "$(dirname "$PASSWORD_FILE")"
if [[ "$REQUIRE_MOUNT" != "true" ]]; then
  mkdir -p "$REPO_PATH"
fi

# Generate password if it doesn't exist
if [ ! -f "$PASSWORD_FILE" ]; then
    echo "Generating repository password..."
    openssl rand -base64 32 > "$PASSWORD_FILE"
    chmod 600 "$PASSWORD_FILE"
    echo "Password saved to: $PASSWORD_FILE"
    echo "IMPORTANT: Save this password in a secure location!"
fi

# Set password environment variable
KOPIA_PASSWORD="$(cat "$PASSWORD_FILE")"
export KOPIA_PASSWORD
export KOPIA_CONFIG_PATH="$CONFIG_FILE"

# Always disconnect and reconnect to ensure we're using the correct repository
if kopia repository status &>/dev/null; then
    echo "Disconnecting from current repository..."
    kopia repository disconnect || true
fi

case "$REPO_TYPE" in
    filesystem)
        echo "Connecting to filesystem repository at $REPO_PATH"
        kopia repository connect filesystem --path="$REPO_PATH" --config-file="$CONFIG_FILE" || {
            echo "Failed to connect to existing repository, creating new one..."
            kopia repository create filesystem --path="$REPO_PATH" --config-file="$CONFIG_FILE"
        }
        ;;
    *)
        echo "Repository type $REPO_TYPE requires manual setup"
        echo "Please run: kopia repository connect $REPO_TYPE --url=<your-repo-url>"
        exit 1
        ;;
esac

echo "Repository initialization complete!"

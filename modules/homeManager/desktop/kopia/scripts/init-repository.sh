#!/usr/bin/env bash
set -euo pipefail

# Kopia repository initialization script

REPO_TYPE="${1:-filesystem}"
REPO_PATH="${2:-$HOME/.local/share/kopia-repository}"
CONFIG_FILE="${3:-$HOME/.config/kopia/repository.config}"
PASSWORD_FILE="${4:-$HOME/.config/kopia/repository.password}"

echo "Initializing Kopia repository..."
echo "Type: $REPO_TYPE"
echo "Path: $REPO_PATH"
echo "Config: $CONFIG_FILE"

# Create directories
mkdir -p "$(dirname "$CONFIG_FILE")"
mkdir -p "$(dirname "$PASSWORD_FILE")"
mkdir -p "$REPO_PATH"

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

# Check if repository is already connected
if kopia repository status &>/dev/null; then
    echo "Repository already connected"
    exit 0
fi

case "$REPO_TYPE" in
    filesystem)
        if [ ! -f "$REPO_PATH/kopia.repository" ]; then
            echo "Creating new filesystem repository at $REPO_PATH"
            kopia repository create filesystem --path="$REPO_PATH" --config-file="$CONFIG_FILE"
        else
            echo "Connecting to existing filesystem repository at $REPO_PATH"
            kopia repository connect filesystem --path="$REPO_PATH" --config-file="$CONFIG_FILE"
        fi
        ;;
    *)
        echo "Repository type $REPO_TYPE requires manual setup"
        echo "Please run: kopia repository connect $REPO_TYPE --url=<your-repo-url>"
        exit 1
        ;;
esac

echo "Repository initialization complete!"

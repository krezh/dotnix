#!/usr/bin/env bash
set -euo pipefail

# Kopia maintenance script

CONFIG_FILE="$1"
PASSWORD_FILE="$2"

echo "Starting Kopia repository maintenance..."

# Set environment variables
KOPIA_PASSWORD="$(cat "$PASSWORD_FILE")"
export KOPIA_PASSWORD
export KOPIA_CONFIG_PATH="$CONFIG_FILE"

notify() {
  local title="$1" message="$2" urgency="${3:-normal}"
  if [[ $urgency == "critical" ]]; then
    notify-send "$title" "❌ $message" --icon=dialog-error --urgency="$urgency"
  else
    notify-send --transient -t 5000 "$title" "✅ $message" --icon=dialog-information --urgency="$urgency"
  fi
}

# Run full maintenance
echo "Running full maintenance..."

if kopia maintenance run --full --safety=full --config-file="$CONFIG_FILE"; then
    echo "Maintenance completed successfully"
else
    echo "Maintenance failed"
    notify "Kopia Maintenance" "Maintenance failed" "critical"
fi

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

# Run full maintenance
echo "Running full maintenance..."
kopia maintenance run --full --safety=none --config-file="$CONFIG_FILE"

MAINTENANCE_STATUS=$?

if [ $MAINTENANCE_STATUS -eq 0 ]; then
    echo "Maintenance completed successfully"
else
    echo "Maintenance failed with status: $MAINTENANCE_STATUS"
fi

exit $MAINTENANCE_STATUS

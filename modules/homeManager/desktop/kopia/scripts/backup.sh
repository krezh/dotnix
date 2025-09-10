#!/usr/bin/env bash
set -euo pipefail

# Kopia backup script

BACKUP_NAME="$1"
CONFIG_FILE="$2"
PASSWORD_FILE="$3"
shift 3
BACKUP_PATHS=("$@")

echo "Starting backup: $BACKUP_NAME"
echo "Paths: ${BACKUP_PATHS[*]}"

# Set environment variables
KOPIA_PASSWORD="$(cat "$PASSWORD_FILE")"
export KOPIA_PASSWORD
export KOPIA_CONFIG_PATH="$CONFIG_FILE"

# Send start notification
notify-send "Kopia Backup" "Starting backup: $BACKUP_NAME" --icon=document-save --urgency=low

# Create snapshot
echo "Creating snapshot..."
kopia snapshot create "${BACKUP_PATHS[@]}" \
    --description="Automated backup: $BACKUP_NAME" \
    --tags="type:automated,name:$BACKUP_NAME" \
    --config-file="$CONFIG_FILE"

BACKUP_STATUS=$?

if [ $BACKUP_STATUS -eq 0 ]; then
    echo "Backup completed successfully"
    notify-send "Kopia Backup" "✅ Backup completed successfully: $BACKUP_NAME" --icon=dialog-ok --urgency=normal
else
    echo "Backup failed with status: $BACKUP_STATUS"
    notify-send "Kopia Backup" "❌ Backup failed: $BACKUP_NAME" --icon=dialog-error --urgency=critical
fi

exit $BACKUP_STATUS

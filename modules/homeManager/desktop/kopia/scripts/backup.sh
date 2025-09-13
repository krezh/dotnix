#!/usr/bin/env bash
set -euo pipefail

BACKUP_NAME="$1"
CONFIG_FILE="$2"
PASSWORD_FILE="$3"
JSON_CONFIG="$4"


KOPIA_PASSWORD="$(cat "$PASSWORD_FILE")"
export KOPIA_PASSWORD
KOPIA_PARALLEL=4

mapfile -t BACKUP_PATHS < <(jq -r '.paths[]' "$JSON_CONFIG")
mapfile -t EXCLUDES < <(jq -r '.exclude[]?' "$JSON_CONFIG")
COMPRESSION=$(jq -r '.compression' "$JSON_CONFIG")
RETENTION_DAILY=$(jq -r '.retentionPolicy.keepDaily // empty' "$JSON_CONFIG")
RETENTION_WEEKLY=$(jq -r '.retentionPolicy.keepWeekly // empty' "$JSON_CONFIG")
RETENTION_MONTHLY=$(jq -r '.retentionPolicy.keepMonthly // empty' "$JSON_CONFIG")
RETENTION_ANNUAL=$(jq -r '.retentionPolicy.keepAnnual // empty' "$JSON_CONFIG")

KOPIA_POLICY_ARGS=()
[[ -n "$COMPRESSION" && "$COMPRESSION" != "null" ]] && KOPIA_POLICY_ARGS+=("--compression=$COMPRESSION")
[[ -n "$RETENTION_DAILY" ]] && KOPIA_POLICY_ARGS+=("--keep-daily=$RETENTION_DAILY")
[[ -n "$RETENTION_WEEKLY" ]] && KOPIA_POLICY_ARGS+=("--keep-weekly=$RETENTION_WEEKLY")
[[ -n "$RETENTION_MONTHLY" ]] && KOPIA_POLICY_ARGS+=("--keep-monthly=$RETENTION_MONTHLY")
[[ -n "$RETENTION_ANNUAL" ]] && KOPIA_POLICY_ARGS+=("--keep-annual=$RETENTION_ANNUAL")
for pattern in "${EXCLUDES[@]}"; do [[ -n "$pattern" ]] && KOPIA_POLICY_ARGS+=("--add-ignore=$pattern"); done

notify() {
  local title="$1" message="$2" urgency="${3:-normal}"
  if [[ $urgency == "critical" ]]; then
    notify-send "$title" "❌ $message" --icon=dialog-error --urgency="$urgency"
  else
    notify-send -t 5000 "$title" "✅ $message" --icon=dialog-information --urgency="$urgency"
  fi
}

if kopia policy set "${KOPIA_POLICY_ARGS[@]}" --config-file="$CONFIG_FILE" "${BACKUP_PATHS[@]}" \
  && kopia snapshot create "${BACKUP_PATHS[@]}" \
      --description="Automated backup: $BACKUP_NAME" \
      --tags="type:automated,name:$BACKUP_NAME" \
      --config-file="$CONFIG_FILE" \
      --parallel="$KOPIA_PARALLEL"; then
  notify "Kopia Backup" "Backup completed successfully: $BACKUP_NAME"
  exit 0
else
  notify "Kopia Backup" "Backup failed: $BACKUP_NAME" "critical"
  exit 1
fi

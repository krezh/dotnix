#!/usr/bin/env bash
set -euo pipefail

if (( $# != 3 )); then
    echo "Usage: $0 <app_name> <app_url> <icon_path>"
    exit 1
fi

APP_NAME="$1"
APP_URL="$2"
ICON_PATH="$3"

if [ -f "$ICON_PATH" ]; then
  exit 0
fi

echo "[webapps] fetching favicon for '$APP_NAME'"
mkdir -p "$(dirname "$ICON_PATH")"

FAVICON_URL="https://favicon.vemetric.com/$APP_URL?size=128&format=png"
TMP_FILE="${ICON_PATH}.tmp"
trap 'rm -f "$TMP_FILE"' EXIT

if curl -fsSL "$FAVICON_URL" -o "$TMP_FILE" && [ -s "$TMP_FILE" ]; then
    echo "[webapps] successfully fetched image for $APP_NAME"
    mv "$TMP_FILE" "$ICON_PATH"
else
    echo "[webapps] failed to fetch icon for '$APP_NAME'"
    exit 1
fi

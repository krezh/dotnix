# Gracefully close all windows then exit niri.
# Similar to hyprshutdown: sends close to each window, waits for them to exit,
# then quits the compositor.

TIMEOUT=10 # seconds to wait for windows to close
POLL_INTERVAL=0.2

close_all_windows() {
  local ids
  ids=$(niri msg -j windows | jq -r '.[].id')
  if [[ -z "$ids" ]]; then
    return 0
  fi
  for id in $ids; do
    niri msg action close-window --id "$id" 2>/dev/null || true
  done
}

wait_for_windows() {
  local elapsed=0
  while true; do
    local count
    count=$(niri msg -j windows | jq 'length')
    if [[ "$count" -eq 0 ]]; then
      return 0
    fi
    if (($(echo "$elapsed >= $TIMEOUT" | bc -l))); then
      return 1
    fi
    sleep "$POLL_INTERVAL"
    elapsed=$(echo "$elapsed + $POLL_INTERVAL" | bc -l)
  done
}

notify-send -t 3000 "Niri" "Closing all windows…"

close_all_windows

if wait_for_windows; then
  niri msg action quit --skip-confirmation
else
  remaining=$(niri msg -j windows | jq -r '.[].title // .[].app_id' | head -5)
  notify-send -u critical -t 5000 "Niri Shutdown" "Windows refused to close:\n${remaining}\n\nForce quit with Mod+Shift+E"
fi

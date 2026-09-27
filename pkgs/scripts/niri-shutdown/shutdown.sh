#!/usr/bin/env bash

set -u

foreground=${NIRI_SHUTDOWN_FOREGROUND:-0}
args=()

for arg in "$@"; do
  case "$arg" in
    --no-fork)
      foreground=1
      ;;
    --help | -h)
      exec "$NIRI_SHUTDOWN_UI" "$@"
      ;;
    *)
      args+=("$arg")
      ;;
  esac
done

if [[ "$foreground" != 1 ]]; then
  NIRI_SHUTDOWN_FOREGROUND=1 setsid "$0" "${args[@]}" </dev/null >/dev/null 2>&1 &
  exit 0
fi

if [[ -z ${XDG_RUNTIME_DIR:-} || -z ${NIRI_SOCKET:-} ]]; then
  printf '%s\n' "niri-shutdown: cannot run outside a Niri session" >&2
  exit 1
fi

instance_dir=$(mktemp -d "$XDG_RUNTIME_DIR/hypr/niri-shutdown.XXXXXX")
instance=${instance_dir##*/}
socket_path="$instance_dir/.socket.sock"

python3 "$NIRI_SHUTDOWN_ADAPTER" "$socket_path" &
adapter_pid=$!

cleanup() {
  kill "$adapter_pid" 2>/dev/null || true
  wait "$adapter_pid" 2>/dev/null || true
  rm -rf "$instance_dir"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

printf '%s\n%s\n' "$adapter_pid" "${WAYLAND_DISPLAY:-wayland-0}" >"$instance_dir/hyprland.lock"

for ((attempt = 0; attempt < 50; attempt++)); do
  [[ -S "$socket_path" ]] && break
  kill -0 "$adapter_pid" 2>/dev/null || {
    printf '%s\n' "niri-shutdown: IPC adapter failed to start" >&2
    exit 1
  }
  sleep 0.01
done

if [[ ! -S "$socket_path" ]]; then
  printf '%s\n' "niri-shutdown: timed out starting IPC adapter" >&2
  exit 1
fi

HYPRLAND_INSTANCE_SIGNATURE=$instance "$NIRI_SHUTDOWN_UI" --no-fork "${args[@]}"

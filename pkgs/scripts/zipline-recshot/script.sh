#!/usr/bin/env bash

set -eEuo pipefail

ZIPLINE_AUTH_TOKEN_FILE="/home/krezh/.config/flameshot/zipline-token"
ZIPLINE_URL="https://zipline.talos.plexuz.xyz"
ZIPLINE_USE_ORIGINAL_NAME=true
SAVE_PATH="/tmp"
imageExt="png"
videoExt="mp4"
dependencies=(hyprctl jq slurp wl-screenrec grim curl wl-copy notify-send hyprpicker)

show_help() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Options:"
  echo "  -u, --url                Set the Zipline URL"
  echo "  -t, --token              Set the path to the Zipline auth token file"
  echo "  -m, --mode               Set the mode (image-area, image-window, image-full, video-area, video-window, video-full)"
  echo "  -p, --path               Set the save path for files (default: ${SAVE_PATH})"
  echo "  -o, --original-name      Use original file name in Zipline (true/false, default: ${ZIPLINE_USE_ORIGINAL_NAME})"
  echo "  -h, --help               Show this help message and exit"
  echo ""
  echo "Example:"
  echo "  $0 -u https://zipline.example.com -t /path/to/token -m image-area -p /tmp -o true"
}

generate_file_name() {
  date +%Y-%m-%d_%H-%M-%S
}

notify() { notify-send -t 5000 "Zipline Recshot" "$@"; }

upload_to_zipline() {
  local uploadFileUrl
  if uploadFileUrl=$(curl --fail -SsL \
    -H "authorization: $(cat "${ZIPLINE_AUTH_TOKEN_FILE}")" \
    -F "file=@${1}" \
    -H 'content-type: multipart/form-data' \
    -H "x-zipline-original-name: ${2}" \
    "${ZIPLINE_URL}/api/upload"); then
      uploadFileUrl=$(echo -n "${uploadFileUrl}" | jq -r .files[0].url | tr -d '\n')
      printf "%s" "${uploadFileUrl}" | wl-copy
      echo "Uploaded: ${uploadFileUrl}"
      notify "Upload successful"
      return 0
    else
      echo "Error uploading file to Zipline." >&2
      notify "Error uploading file to Zipline."
      exit 1
  fi
}

check_var() {
  if [ -z "$ZIPLINE_URL" ]; then
    echo "Error: ZIPLINE_URL is not set."
    exit 1
  fi

  if [ -z "$ZIPLINE_AUTH_TOKEN_FILE" ]; then
    echo "Error: ZIPLINE_AUTH_TOKEN_FILE is not set."
    exit 1
  fi

  if [ ! -f "$ZIPLINE_AUTH_TOKEN_FILE" ]; then
    echo "Error: ZIPLINE_AUTH_TOKEN_FILE does not exist."
    exit 1
  fi
}

check_dependencies() {
  for cmd in "${dependencies[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
      echo "Error: $cmd is not installed." >&2
      exit 1
    fi
  done
}

active_window() {
  hyprctl -j activewindow | \
    jq -r '"\(.at[0]),\(.at[1]) \(.size[0])x\(.size[1])"'
}

active_monitor() {
  hyprctl -j activeworkspace | \
    jq -r '.monitor'
}

check_recording() {
  local file
	if pgrep -x "wl-screenrec" > /dev/null; then
      file=$(pgrep -a "wl-screenrec" | awk -F'-f ' '{print $2}')
			pkill -INT -x "wl-screenrec"
			notify "Screen recording saved"
      echo "Recording saved to: ${file}"
      upload_to_zipline "${file}" "${ZIPLINE_USE_ORIGINAL_NAME}"
			exit 0
	fi
}

killHyprpicker() {
  if pidof hyprpicker >/dev/null; then
    pkill hyprpicker
  fi
}

slurp_freeze() {
  local output
  if output=$(slurp); then
    echo -n "$output"
    killHyprpicker
    return 0
  else
    killHyprpicker
    return 1
  fi
}

video_area() {
  local geometry
  geometry="$(slurp)" || exit 1
  notify "Recording area, run again to stop"
  wl-screenrec --low-power=off -g "${geometry}" -f "${1}"
}

video_window() {
  notify "Recording window, run again to stop"
  wl-screenrec --low-power=off -g "$(active_window)" -f "${1}"
}

video_full() {
  notify "Recording fullscreen, run again to stop"
  wl-screenrec --low-power=off -o "$(active_monitor)" -f "${1}"
}

screenshot_area() {
  local geometry
  hyprpicker -r -z & sleep 0.2
  geometry="$(slurp_freeze)" || exit 1
  grim -g "${geometry}" "${1}"
}

screenshot_window() { grim -g "$(active_window)" "${1}"; }

screenshot_full() { grim "${1}"; }

while [[ "$#" -gt 0 ]]; do
  case $1 in
    -u|--url) ZIPLINE_URL="$2"; shift ;;
    -t|--token) ZIPLINE_AUTH_TOKEN_FILE="$2"; shift ;;
    -m|--mode) MODE="$2"; shift ;;
    -p|--path) SAVE_PATH="$2"; shift ;;
    -o|--original-name) ZIPLINE_USE_ORIGINAL_NAME="$2"; shift ;;
    -h|--help) show_help; exit 0 ;;
    *) echo "Unknown parameter passed: $*"; show_help; exit 1 ;;
  esac
  shift
done

check_var
check_dependencies

if [[ "${MODE}" == video-* ]]; then
  check_recording
fi

filePath="${SAVE_PATH}/$(generate_file_name)"

case "${MODE}" in
  image-area) screenshot_area "${filePath}.${imageExt}" \
    && upload_to_zipline "${filePath}.${imageExt}" "${ZIPLINE_USE_ORIGINAL_NAME}" ;;
  image-window) screenshot_window "${filePath}.${imageExt}" \
    && upload_to_zipline "${filePath}.${imageExt}" "${ZIPLINE_USE_ORIGINAL_NAME}" ;;
  image-full) screenshot_full "${filePath}.${imageExt}" \
    && upload_to_zipline "${filePath}.${imageExt}" "${ZIPLINE_USE_ORIGINAL_NAME}" ;;
  video-area) video_area "${filePath}.${videoExt}" ;;
  video-window) video_window "${filePath}.${videoExt}" ;;
  video-full) video_full "${filePath}.${videoExt}" ;;
esac
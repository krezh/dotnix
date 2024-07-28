#!/bin/bash

icon_dir="$XDG_CONFIG_HOME/dunst/icons"
volume=0

get_volume() {
  volume="$(wpctl get-volume @DEFAULT_AUDIO_SINK@)" &&
  volume=${volume#Volume: *} &&
  case "$volume" in (*MUTED*) volume=0;; esac &&
  printf "%0.0f\n" "${volume%% *}e+2"
}

send_notification() {
  volume=$(get_volume)
  if [ "$volume" -lt 0 ]; then
    icon="$icon_dir/volume-off.svg"
  elif [ "$volume" -lt 10 ]; then
    icon="$icon_dir/volume-low.svg"
  elif [ "$volume" -lt 30 ]; then
    icon="$icon_dir/volume-low.svg"
  elif [ "$volume" -lt 70 ]; then
    icon="$icon_dir/volume-medium.svg"
  elif [ "$volume" -lt 101 ]; then
    icon="$icon_dir/volume-high.svg"
  fi

  dunstify -a "Volume" -u normal -i "$icon" -r "1231" -h int:value:"$volume" "Volume: ${volume}%"
}

undo_mute() {
  volume=$(get_volume)
  if [ "$volume" = 0 ]; then
    wpctl set-mute @DEFAULT_AUDIO_SINK@ 0
  fi
}

case $1 in
  up)
    undo_mute
	  wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+
	  send_notification
  ;;
  down)
    undo_mute
	  wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%-
	  send_notification
  ;;
  mute)
	  wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
    if [[ $(get_volume) = 0 ]]; then
      icon="$icon_dir/volume-off.svg"
      dunstify -i "$icon" -r 1231 -a "Volume" "Volume: Muted"
    else
      send_notification
    fi
  ;;
esac
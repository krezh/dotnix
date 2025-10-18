#!/usr/bin/env bash

icon_dir="$XDG_CONFIG_HOME/dunst/icons"
steps=5

send_notification () {
  brightness=$(brightnessctl i | rg -oP '\(\K[^%\)]+')

  if [ "$brightness" = "0" ]; then
    icon="$icon_dir/brightness-1.svg"
  elif [ "$brightness" -lt "15" ]; then
    icon="$icon_dir/brightness-2.svg"
  elif [ "$brightness" -lt "30" ]; then
    icon="$icon_dir/brightness-3.svg"
  elif [ "$brightness" -lt "45" ]; then
    icon="$icon_dir/brightness-4.svg"
  elif [ "$brightness" -lt "60" ]; then
    icon="$icon_dir/brightness-5.svg"
  elif [ "$brightness" -lt "75" ]; then
    icon="$icon_dir/brightness-6.svg"
  else
    icon="$icon_dir/brightness-7.svg"
  fi

  dunstify \
    -a Brightness \
    -i "$icon" \
    -t 2000 \
    -r 500 \
    -h int:value:"$brightness" \
    "Brightness: $brightness"
}

case $1 in
  up)
    brightnessctl set "${steps:-5}%+" -q
    send_notification
  ;;
  down)
    brightnessctl set "${steps:-5}%-" -n 5% -q
    send_notification
  ;;
esac

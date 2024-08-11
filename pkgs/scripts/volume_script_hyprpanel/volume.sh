#!/bin/bash

steps=5

case $1 in
  up)
	  wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ "${steps}%+"
  ;;
  down)
	  wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ "${steps}%-"
  ;;
  mute)
	  wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
  ;;
esac
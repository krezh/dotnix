#!/bin/bash

steps=5

case $1 in
  up)
    brightnessctl set "${steps:-5}%+" -q
  ;;
  down)
    brightnessctl set "${steps:-5}%-" -n 5% -q
  ;;
esac
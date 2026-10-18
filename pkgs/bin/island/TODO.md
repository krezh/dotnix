# island TODO

## Integration
- [ ] Make the terminal for terminal apps configurable (now `xdg-terminal-exec`)
- [ ] Another bar's exclusive zone (Noctalia) moves the island down, so `Island::to_screen` is off
      by that much

## Open questions
- [ ] `headsetcontrol` and `molt check` still run as subprocesses
- [ ] Tooltips on chart cards (value under the pointer)
- [ ] DDC/CI brightness has no change events, so its OSD only follows `island ctl brightness`

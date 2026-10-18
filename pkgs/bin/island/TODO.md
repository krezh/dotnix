# island TODO

## Features

- [ ] Network: Wi-Fi scan/connect/forget, ethernet status, VPN toggles (NetworkManager over D-Bus);
      authentication, saved connections, VPN secrets and cancellation
- [ ] Bluetooth: power, scan, pair/connect/forget, device battery (BlueZ over D-Bus);
      pairing confirmation and PIN prompts through a BlueZ agent
- [ ] Battery charge/status and power profile selection (UPower, power-profiles-daemon)
- [ ] Night light / color temperature: compositor-supported control and coordination with other
      color-temperature services
- [ ] Weather: choose the provider, location source and forecast scope
- [ ] Calendar events in the clock panel: choose the event source (local ICS, CalDAV or another service)
- [ ] Keyboard layout indicator and switcher through the compositor backend; reflect external changes
- [ ] Screen recording through chomp: indicator and controls follow chomp's actual state, including
      recordings started outside island
- [ ] Wallpaper picker through awww, with per-output selection

## Integration

- [ ] Another bar's exclusive zone (Noctalia) moves the island down, so `Island::to_screen` is off
      by that much; use the actual compositor placement offset

## Open questions

- [ ] `headsetcontrol` and `molt check` still run as subprocesses; assess stable native APIs before
      replacing them
- [ ] DDC/CI brightness has no change events, so its OSD only follows `island ctl brightness`;
      decide polling cadence and whether externally detected changes show an OSD

# island

A dynamic-island status bar, notification daemon and shell for Hyprland, written in Rust with GPUI
(`gpui-pre` / `gpui-pre-platform` pinned at `=0.3.9`, patched for runtime layer switching). Built
with `nix build .#island`, run as the `homeModules.island` systemd user service.

## Ground rules

- Hyprland first. Compositors are backends behind `compositor::Compositor`; a niri backend is
  welcome later, nothing outside `src/compositor/` names a compositor.
- Comments are doc comments only.
- Catppuccin Mocha everywhere; accent colours are welcome. Translucent, blurred body; no opaque
  slabs. Hover and selection are translucent white tints (`ui::hover_fill`, `ui::selected_fill`),
  never grey fills.
- Type: four sizes only (`SMALL_SIZE` 13, `TEXT_SIZE` 14, `TITLE_SIZE` 16, `HEADING_SIZE` 18).
  Medium is the base weight, semibold for titles, values and names, bold only for the clock and
  today's date. Text is `text`; `subtext1` only for metadata (timestamps, sources, notes, empty
  states); `overlay2` only for inactive things (muted, disabled, placeholder, other months).
- Shapes: the island uses config `radius` (from `var.rounding`) with `power` 2, i.e. circular; a
  power-4 squircle at the same radius reads like a circle half its size. Inside it corners are
  circular: panels and cards 16, tooltips and buttons 10, tabs and list rows 8, tracks and knobs
  pills (`ui::Radius`). Every shape goes through `ui::shape` / `ui::outlined`, which paint
  anti-aliased corner sprites plus pixel-snapped edges; GPUI paths are only 4x MSAA and look
  stepped, so never fill shapes with paths.
- Motion over snapping: switch knobs and slider fills follow springs (the island's shape spring).
  Sliders have no knob; the fill is the handle. Toggles signal state with the track only.
- Everything is a reusable component: panels, cards, graphs, toggles, buttons, lists, tooltips. A
  new panel is a list of components, never hand-laid-out.
- Sizes come from standardized component sizes, not per-panel numbers. Panels are measured once
  (at startup, or when their structure changes) and frozen; values changing never resizes anything.
  The island morphs to whatever size the panel needs, wider than the bar if needed. No scrolling
  except unbounded lists.
- Modular: a feature is one folder or file under `src/features/` implementing `Feature`, listed once
  in `features![…]`; nothing else names it. The bar has three sections (left, middle, right), each a
  list of widget names from the config file.
- Workers run under `supervisor::supervise` and restart on a crash; a source that goes away (sound
  server, D-Bus peer) is reconnected by the worker itself and shown as an error, never hidden.
- Native backends: talk to services through libraries and D-Bus, not by piping out to binaries
  (libpulse for audio, logind / DDC for brightness, D-Bus for MPRIS, tray and notifications, an HTTP
  client for cover art). Launching apps is the only process spawning by design. One-shot D-Bus
  proxies go through `dbus::uncached`. Remaining exceptions are listed under TODO.
- Testing: `nix run .#island` after stopping the `island` user service; `island logs` follows it.
  Never move the cursor, click, scroll or inject input in the live session. Never activate, switch
  or reload the system.

## Architecture

| Path | Contents |
|---|---|
| `main.rs` | Argument parsing, logging, app start |
| `shell.rs` | One island per output, hotplug, notification routing, feature startup |
| `island.rs` | Per-output state, springs, measurement, layer switching, render |
| `features/` | The `Feature` framework (`mod.rs`) and every feature: `clock`, `notifications/`, `workspaces`, `tray/`, `audio/`, `media/`, `molt`, `headset`, `zowie`, `caffeine` |
| `compositor/` | The `Compositor` backend trait and `hyprland.rs` |
| `panel.rs`, `panels.rs` | Panel framework (header, blocks) and island's own panels (system, menu, launcher, OSD) |
| `bar.rs` | Collapsed bar, widget placement and folding, tab strip |
| `supervisor.rs`, `logging.rs` | Restarting workers; tracing, the log socket and `island logs` |
| `dbus.rs`, `control.rs` | island's own D-Bus name and `island ctl`; shared D-Bus helpers |
| `effects.rs`, `icons.rs`, `sys.rs` | Blur and idle inhibit, icon lookup, system sampler |
| `wm.rs`, `glyphs.rs` | Compositor-neutral monitor and workspace model; the shared icon table (a GPUI global) |
| `components/` | Reusable UI parts: button, card, chart, marquee, slider, tooltip, aura, … |

### GPUI gotchas

1. Unfocused windows are capped at about 30 fps; set `inactive_frame_interval: None`.
2. `KeyboardInteractivity` can't change at runtime; the bar uses `None`. Anything that needs typing
   (launcher) needs its own surface.
3. Element hover goes stale after the pointer leaves the surface; drive hover from
   `window.is_window_hovered()`.
4. Rounded rectangles are circular only; squircles and charts are `canvas` paths.
5. `BoxShadow` darkens the translucent body; the island uses an outside-only ring shadow.
6. Clipping is rectangles only.
7. `Entity::update` on the async context returns `()`; zbus reports a taken name as `NameTaken`.
8. The layer can only change through the local `set_layer` patch (`patches/`); regenerate it when
   bumping GPUI.

## TODO

### Features
- [ ] The OSD as a feature pop-up instead of its own panel (brightness is still core)
- [ ] Menus: a menu concept separate from panels. A tray item's right-click menu (and any feature's)
      bulges out of the island below the clicked widget instead of morphing the whole island: the
      bar stays, a small body grows out of the island's lower edge in the same material, picks and
      submenus happen inside it, and it closes on a pick, a click elsewhere or Escape. Panels keep
      the full morph. Joined to the island: the body grows out of its lower edge with curved inner
      corners where they meet, one continuous shape. Centred under the pointer, or the clicked icon
      when opened from the keyboard, and pushed sideways so it never reaches past the island's
      left or right edge. It has a fixed maximum width; longer labels are cut short.
      Needs a compound shape for the island, its blur region and its input region.

### Verify live
- [ ] Launcher morph and typing
- [ ] Hover fills, switch and slider springs, volume scroll (need real pointer input)
- [ ] Brightness on thor needs DDC/CI: `hardware.i2c.enable = true` and the user in `i2c`

### Integration
- [ ] Hyprland binds for `island ctl` (launcher, volume and brightness keys, media keys)
- [ ] Terminal apps launch with `xdg-terminal-exec`; make the terminal configurable
- [ ] The island assumes its window starts at the top of its monitor; another bar's exclusive zone
      (Noctalia) moves it down, so `Island::to_screen` is off by that much

### Open questions
- [ ] `headsetcontrol` has no library (its device protocols live in the C program); it is still a
      subprocess. `molt check` too, unless molt grows a library crate.
- [ ] Tooltips on chart cards (value under the pointer)
- [ ] Brightness has no change events on DDC/CI, so its OSD only follows `island ctl brightness`

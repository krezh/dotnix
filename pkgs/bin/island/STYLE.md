# island style guide

island should read and look like one thing, whichever feature is on screen. When a feature needs
something this guide does not cover, add it here and to the shared code, not to the feature.

## Words

- Sentence case for titles, labels and buttons. Names keep their capitals: `Open Molt`, `Claude Code`.
- `·` separates parts of a line: `Connected · 2h 5m`.
- `…` for anything still going on or cut short: `Connecting…`. Never three dots.
- `—` stands for a value that is missing.
- A failure says what could not be done, starting `cannot`: `cannot read the launch history: …`.
  Capitalize it only when it is shown as a sentence of its own: `Cannot connect.`
- Write counts with `units::plural`.

## Numbers

Always format through `units.rs`, so a value reads the same everywhere.

| What           | Function       | Reads as                                 |
| -------------- | -------------- | ---------------------------------------- |
| Size           | `units::bytes` | `512 B`, `4.2 kB`, `12 MB`, `1.5 GB`     |
| Speed          | `units::rate`  | `512 B/s`, `1.5 MB/s`                    |
| Link speed     | `units::link`  | `866 Mb/s`, `2.5 Gb/s`                   |
| Length of time | `units::span`  | `<1m`, `4m`, `2h 5m`, `3d 4h`            |
| Time since     | `units::ago`   | `just now`, `4m ago`, `2h ago`, `3d ago` |

- Sizes count in steps of 1024 and keep one decimal only under 10.
- Other units take a space: `12 ms`, `2.4 GHz`, `-60 dBm`, `3.92 V`.
- Time units are one letter with no space. So is a percentage: `85%`.
- A figure's unit may be drawn smaller, but it keeps these spacing rules: `15%`, `74°C`, `12 ms`.
- On a notification card, time since is short: `now`, `5m`, `2h`.

## Color

- Colors come from `theme.rs`. Never write a color in a feature.
- Use the whole Catppuccin Mocha palette, not mostly blue: give different things different colors.
- `ACCENT` means active or interactive; `CRITICAL` means something is wrong.
- Severity runs green, then yellow, then `CRITICAL`.
- Everything that reacts to the pointer hovers in `ui::hover_fill`, the same accent blue.
- What is currently selected uses `ui::selected_fill`, the gray. It marks a choice, not the pointer.
- Something standing for an app takes `theme::app_color`, so it keeps its color everywhere.

## Type

- `FONT` for everything, with tabular numbers, as `ui::base` sets up. `MONO` for code and commands.
- Sizes come from `ui.rs`: `SMALL_SIZE`, `TEXT_SIZE`, `TITLE_SIZE`, `HEADING_SIZE`.

## Space and shape

- Spacing and heights come from `ui.rs`: `INSET`, `BLOCK_GAP`, `CARD_PAD`, `CONTROL_H`, `HEADER_H`.
- Corners come from `ui::Radius`, picked by what the shape is, never by a number.
- Everything clickable draws through `ui::control`, so all of it hovers the same way.
- Controls keep their established shapes. Do not swap one for a pill to make it fit.
- Layouts must still work with many items, not just the few in a mockup.
- No visible scrollbars. Scrolling regions use soft edge fades (`components::edges`) instead.

## Motion

- Every state change animates.
- Text that can change while shown, such as numbers, units, states and device names, goes through
  `components::motion::text` or `motion::steady` with a stable, unique id, so it morphs into its new
  value. Only fixed labels may be plain text.
- Continuity over crossfades: keep what persists and move it. Never fade a whole view out to empty
  and back, never let content blink to nothing, and never show two copies of the same text at once.
- Springs do not overshoot.
- Motion matches the island's own morph: it grows evenly from its source, with content held in
  place and uncovered.
- Something busy shows the energy border.

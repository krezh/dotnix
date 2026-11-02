# island

## Building

- Build and test only with `nix build .#island` from the repo root; its check phase runs the tests.
  Never run cargo directly, not even through `nix develop`.

## Structure

- `mod.rs` only collects: `mod` lines and `pub use` re-exports, nothing else. No `impl` blocks
  (not even `impl Feature`), structs, consts, functions or macros. Code goes in small files named
  for what they do (`widget.rs`, `panel.rs`, `state.rs`, …); a feature's `impl Feature` goes in its
  `feature.rs`.
- Tests live in a sibling `<name>_test.rs`, wired with `#[cfg(test)] #[path = "<name>_test.rs"] mod tests;`.

## Toolkit boundary

- `Feature` is what runs without drawing: worker, state, settings, commands. `Surface` is what
  shows: `Local`, widgets, panels, keys.
- Feature logic never names GPUI. It takes `Key`, `Measure` and `Ui`, and gets `SharedString`,
  `SpringConfig` and `Element` from island's own modules. A click reaches the island through `Ui`,
  so a new need there is a new method on `Ui`.
- A running feature lives in `features/instance.rs`, outside any toolkit. Whatever draws the islands
  applies what features send through `Frontend`; `frontend.rs` is GPUI's.
- A bar widget is data too: a feature returns a `BarWidget` built from `Part`s, and `bar.rs` draws
  it. Something with its own drawing gives a `Drawing`, a list of shapes any frontend can paint.
- `stage/` is an island without its drawing: what is open, the shape and fades, the features' parts
  and what each panel shows. `island.rs` is GPUI's drawing, timers and input on top of it.
- `gpu/` draws the same `Stage`, `BarWidget`s and panel blocks with wgpu, chosen with
  `frontend = "wgpu"` in the config or `--frontend wgpu`. A panel is laid out once into sprites; only what moves is made each frame.
- Nothing in `features/` may use GPUI or `island.rs`; `architecture_test.rs` fails otherwise.

## Preview

- `island --preview` runs beside a running island: `profile.rs` gives it its own state, cache,
  runtime folder, bus name and namespace. Something only one island can own, such as a bus name
  or an agent, checks `profile::preview()` and stays with the real one. With no island running,
  it is the real island.

## UI is built from components

- A feature describes its panel from general blocks and components (`Card`, `Grid`, `Item`,
  `Slider`, `Segments`, `Dropdown`, …) and supplies data and handlers. It never draws UI itself:
  no GPUI layout code in features. There is deliberately no escape hatch for hand-drawn blocks.
- Prefer making an existing component more configurable over adding a new one.
- Extract a component only for something genuinely shared or reusable. Drawing that only one
  feature does and is likely to stay unique (the visualizer's bars, workspace dots, tray slots)
  stays in that feature; don't move code out for its own sake.
- Shared code never names a feature: no feature-specific types or blocks in `panel/` or
  `components/`. Components get general names (`pick`, not `device`).
- Every component file opens with a `//!` line saying what it is for. Read it before using one.
- Never stretch a component to a job it was not made for. If nothing fits, stop, propose a new
  component, and ask before building it.
- A part with one owner stays private to that owner. Example: the menu (`menu.rs`) is the
  island's own menu, hanging from the bar; its rows are private to it and panels never use it. A
  pick-one control in a panel is a `Dropdown`.

## Visual design

- `STYLE.md` is the style guide for everything shown: words, numbers, color, type, space and motion.
  Follow it, and extend it when something new needs a rule.
- The owner is precise about visual language. When building from a mockup, match it, then compare
  a screenshot of the result against the mockup and name every difference, rather than reporting
  it done.

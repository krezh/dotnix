# island

## Building

- Build and test only with `nix build .#island` from the repo root; its check phase runs the tests.
  Never run cargo directly, not even through `nix develop`.

## Structure

- `mod.rs` only collects: `mod` lines plus the thinnest glue, such as `impl Feature` with each
  method a one-line call. Real code goes in small files named for what they do (`widget.rs`,
  `panel.rs`, `state.rs`, …).
- Tests live in a sibling `<name>_test.rs`, wired with `#[cfg(test)] #[path = "<name>_test.rs"] mod tests;`.

## UI is built from components

- A feature describes its panel from general blocks and components (`Card`, `Grid`, `Item`,
  `Slider`, `Segments`, `Dropdown`, …) and supplies data and handlers. It never draws UI itself:
  no GPUI layout code in features. There is deliberately no escape hatch for hand-drawn blocks.
- Prefer making an existing component more configurable over adding a new one.
- Extract a component only for something genuinely shared or reusable. Drawing that only one
  feature does and is likely to stay unique (the visualizer's bars, workspace dots, tray slots)
  stays in that feature; don't move code out for its own sake.
- Shared code never names a feature: no feature-specific types or blocks in `panel.rs` or
  `components/`. Components get general names (`pick`, not `device`).
- Every component file opens with a `//!` line saying what it is for. Read it before using one.
- Never stretch a component to a job it was not made for. If nothing fits, stop, propose a new
  component, and ask before building it.
- A part with one owner stays private to that owner. Example: the menu (`menu.rs`) is the
  island's own menu, hanging from the bar; its rows are private to it and panels never use it. A
  pick-one control in a panel is a `Dropdown`.

## Visual design

- The owner is precise about visual language. When building from a mockup, match it, then compare
  a screenshot of the result against the mockup and name every difference, rather than reporting
  it done.

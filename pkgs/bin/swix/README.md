# Swix

Swix is a GTK4 layer-shell update manager for NixOS and Home Manager. It builds
the selected configuration, displays its `dix` closure diff, opens release notes
for changed packages, and activates the exact build that was reviewed.

Swix builds the flake state already present on disk. It does not update
`flake.lock` or fetch a newer revision on the user's behalf.

## Architecture

The NixOS module installs two binaries:

- `swix` is the unprivileged GTK application. It evaluates the selected target
  once, builds that exact derivation, keeps the result alive with a temporary GC
  root, and displays the diff.
- `swix-helper` is a socket-activated root helper. It rejects concurrent
  activation requests, updates the system profile, runs
  `switch-to-configuration switch`, and restores the prior profile if
  activation fails.

The module writes `/etc/swix/swix.toml` and creates `/run/swix.sock` with mode
`0600`, owned by the configured user. Home Manager activation runs directly as
that user and does not use the root helper.

Package names in a report are clickable. Swix delegates release resolution to
`nix-changelog`, which may evaluate or fetch nixpkgs metadata and access package
forges over the network. Successful lookups are cached for the Swix session.

## Security Model

The value of `nixosModules.swix.user` is an administrator trust boundary. That
user can ask the root service to activate a caller-supplied Nix store closure and
must therefore be treated as effectively root-equivalent. The store-path and
activation-script checks protect against mistakes; they are not a privilege
boundary.

The helper intentionally remains root because NixOS profile updates and system
activation require root privileges. Moving authorization to a narrower polkit
flow is future work.

## Configuration

```nix
{
  nixosModules.swix = {
    enable = true;
    user = "alice";
    settings = {
      flakeDir = "/home/alice/dotnix";
      nixosFlake = "workstation";
      homeFlake = null;
      appearance = {
        sansFont = "sans-serif";
        monoFont = "monospace";
        rounding = 15;
      };
    };
  };
}
```

Set `homeFlake` to a standalone `homeConfigurations.<name>` output to expose a
separate Home Manager target. Integrated Home Manager needs no separate target
because it is already part of the NixOS closure.

Override the configured NixOS flake output for one Swix session with:

```sh
swix --host odin
```

Invoking `swix --host` while Swix is already open starts that host's build in
the existing window. It replaces an active build, but is rejected while
activation is running.

## Operation And Recovery

During a build, each derivation reported by Nix receives a permanent build-plan
square. The square stays in its original grid slot and changes color from queued
through building or downloading to complete or failed; completed work does not
disappear. The activity list below names every derivation currently building or
downloading and shows its live phase or byte progress. Active rows retain their
first-seen order and widgets as states and progress change; new work appends.
Build and comparison commands have time limits and can be cancelled from the
loading screen. Cancellation terminates the command's complete process group.
Build failures show the originating builder error and its retained log tail rather
than the final dependency-propagation summary. The read-only error panel is
selectable and scrollable and preserves the `nix log` command for the full build
log.
A reviewed output remains protected from garbage collection until its report is
replaced or Swix exits. Swix also records the active profile used to produce the
report. If that profile changes before activation, switching is rejected and the
report must be rebuilt. The NixOS helper performs this check while holding the
activation lock; Home Manager is checked immediately before its activation
script starts.

NixOS activation accepts only one request at a time; concurrent requests fail
with a retryable error instead of waiting in a queue. The helper bounds captured
command output, gives profile update and activation a shared 30-minute deadline,
and terminates their complete process groups on timeout. If
`switch-to-configuration` fails, the helper gives profile rollback a separate
five-minute deadline, restores the previous persistent system profile, and
reports the failure. Runtime state may still have been changed partially by the
failed activation; inspect the journal and reboot or switch to a known
generation if necessary.

Activation cannot be cancelled or left through Swix navigation after it starts.
The window remains open until the helper or Home Manager activation reports
success or failure.

Useful commands:

```sh
journalctl -u 'swix@*'
systemctl status swix.socket
sudo nixos-rebuild switch --rollback
```

## Development

The package build runs all Rust unit tests for the GTK application and helper:

```sh
nix build .#swix
```

Formatting is covered by the repository treefmt check. Repeated launches reuse
the existing Swix window instead of opening another main window. Rust modules
separate activation, builds, changelogs, bounded command lifecycle,
configuration, Nix progress, report parsing, and focused build-progress,
changelog, and switch UI components.

## Keyboard Controls

- `Tab` / `Shift+Tab`: move through buttons and package changelog links.
- `Enter` / `Space`: activate the focused control.
- `N`: build the NixOS target.
- `M`: build standalone Home Manager when that target is enabled.
- `S`: switch to the reviewed build.
- `Up` / `K`, `Down` / `J`: scroll reports and changelogs.
- `Page Up` / `Page Down`: scroll one page.
- `Home` / `g`, `End` / `Shift+G`: jump to the beginning or end.
- `Left` / `H`: return to the chooser and cancel an active build; disabled
  during activation.
- `Escape`: cancel active build or changelog work and close the window; disabled
  during activation.

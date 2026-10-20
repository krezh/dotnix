<div align="center">

### My NixOS Config Repository

<img src="https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/palette/macchiato.png" width="500"/>

_... managed with Renovate, and GitHub Actions_ 🤖

</div>

<div align="center">

[![built with nix](https://img.shields.io/badge/built_with_nix-blue?logo=nixos&logoColor=white&colorA=363a4f&colorB=74c7ec&style=for-the-badge)](https://builtwithnix.org)
[![Build Nix](https://img.shields.io/github/actions/workflow/status/krezh/dotnix/build-ci.yaml?branch=main&label=&logo=github&colorA=363a4f&colorB=b7bdf8&style=for-the-badge)](https://github.com/krezh/dotnix/actions/workflows/build-ci.yaml)

![Repo Size](https://img.shields.io/github/repo-size/krezh/dotnix?color=ea999c&labelColor=303446&style=for-the-badge&link=https%3A%2F%2Fgithub.com%2Fkrezh%2Fdotnix)
![License](https://img.shields.io/static/v1.svg?label=License&message=GPLv3&logoColor=ca9ee6&colorA=313244&colorB=cba6f7&style=for-the-badge)

</div>

## Molt catalog search

Molt's Home **Search** card discovers packages, NixOS options and Home Manager
options from [search.nixos.org](https://search.nixos.org). With shortcuts enabled,
press **S** on Home; the card remains clickable with shortcuts disabled.
Home sizes itself to show all dashboard rows, with scrolling only when the monitor
is too short to fit them.
Search keeps Back, Close and page navigation in a bottom footer.

Search uses the public `nixos-unstable` catalog, not the repository's locked inputs.
It is read-only: inspect metadata, copy attributes or option paths, and open package
homepages without installing packages or editing configuration. Results are paged
in groups of 50, up to the backend's 10,000-result window. Network or catalog errors
offer a manual **Retry** that rediscovers the current catalog.
The current query's 16 most recently used category/page results are cached in
memory while Search is open. Revisiting a cached category or page does not reload
it. Changing the query clears the cache; **Retry** fetches fresh results.

Build and run the package checks with:

```sh
nix build .#molt --no-link --print-out-paths
```

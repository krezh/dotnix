<div align="center">

### My NixOS Config Repository

<img src="https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/palette/macchiato.png" width="500"/>

_... managed with Renovate, and GitHub Actions_ 🤖

</div>

<div align="center">

[![built with nix](https://img.shields.io/badge/built_with_nix-blue?logo=nixos&logoColor=white&colorA=363a4f&colorB=74c7ec&style=for-the-badge)](https://builtwithnix.org)
[![Renovate](https://img.shields.io/github/actions/workflow/status/krezh/renovate-config/renovate.yaml?branch=main&label=&logo=renovate&colorA=363a4f&colorB=b7bdf8&style=for-the-badge)](https://github.com/krezh/renovate-config/actions/workflows/renovate.yaml)
[![Build Nix](https://img.shields.io/github/actions/workflow/status/krezh/renovate-config/renovate.yaml?branch=main&label=&logo=github&colorA=363a4f&colorB=b7bdf8&style=for-the-badge)](https://github.com/krezh/nix-config/actions/workflows/build-nix.yaml)

![Repo Size](https://img.shields.io/github/repo-size/krezh/nix-config?color=ea999c&labelColor=303446&style=for-the-badge&link=https%3A%2F%2Fgithub.com%2Fkrezh%2Fnix-config)
![License](https://img.shields.io/static/v1.svg?label=License&message=GPLv3&logoColor=ca9ee6&colorA=313244&colorB=cba6f7&style=for-the-badge)

```emerald
digraph ModuleGraph {
	node [fontname=Helvetica shape=box style=filled]
	"s08d9f5avsqs69r1y2gyivxgfwv4a88c-flake.nix" [label=<<B>flake.nix</B><BR/><BR/><I>s08d9f5avsqs69r1y2gyivxgfwv4a88c</I>> fillcolor="#e5dbb2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" [label=<<B>flake.nix#modules.nixos.thor</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/asmedia.nix" [label=<<B>modules/hosts/thor/asmedia.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" [label=<<B>modules/hosts/thor/configuration.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-desktop" [label=<<B>flake.nix#modules.nixos.system-desktop</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" [label=<<B>modules/system/system-types/system-desktop/configuration.nix</B><BR/>flake.modules.nixos.system-desktop<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-base" [label=<<B>flake.nix#modules.nixos.system-base</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" [label=<<B>modules/system/system-types/system-base/nixos.nix</B><BR/>flake.modules.nixos.system-base<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops" [label=<<B>modules/sops</B><BR/><BR/><I>67n5jl06z5k7zk8rl8zklpk61yrhxwdp</I>> fillcolor="#b5e5b2"]
	"67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops/templates" [label=<<B>modules/sops/templates</B><BR/><BR/><I>67n5jl06z5k7zk8rl8zklpk61yrhxwdp</I>> fillcolor="#b5e5b2"]
	"67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops/secrets-for-users" [label=<<B>modules/sops/secrets-for-users</B><BR/><BR/><I>67n5jl06z5k7zk8rl8zklpk61yrhxwdp</I>> fillcolor="#b5e5b2"]
	"56790dvx2fs9dxp488v886ks40y2pcs0-nixos" [label=<<B>nixos</B><BR/><BR/><I>56790dvx2fs9dxp488v886ks40y2pcs0</I>> fillcolor="#e5bbb2"]
	"56790dvx2fs9dxp488v886ks40y2pcs0-nixos/common.nix" [label=<<B>nixos/common.nix</B><BR/><BR/><I>56790dvx2fs9dxp488v886ks40y2pcs0</I>> fillcolor="#e5bbb2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/settings/variables/style.nix" [label=<<B>modules/system/settings/variables/style.nix</B><BR/>flake.modules.generic.var<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.shell" [label=<<B>flake.nix#modules.nixos.shell</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/shell/fish.nix" [label=<<B>modules/programs/shell/fish.nix</B><BR/>flake.modules.nixos.shell<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.catppuccin" [label=<<B>flake.nix#modules.nixos.catppuccin</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/catppuccin/nixos.nix" [label=<<B>modules/programs/catppuccin/nixos.nix</B><BR/>flake.modules.nixos.catppuccin<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-flake.nix#nixosModules.catppuccin" [label=<<B>flake.nix#nixosModules.catppuccin</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos" [label=<<B>modules/nixos</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" [label=<<B>modules/global.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/cursors.nix" [label=<<B>modules/nixos/cursors.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/fcitx5.nix" [label=<<B>modules/nixos/fcitx5.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/gitea.nix" [label=<<B>modules/nixos/gitea.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/grub.nix" [label=<<B>modules/nixos/grub.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/gtk.nix" [label=<<B>modules/nixos/gtk.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/limine.nix" [label=<<B>modules/nixos/limine.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/plymouth.nix" [label=<<B>modules/nixos/plymouth.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/sddm.nix" [label=<<B>modules/nixos/sddm.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/tty.nix" [label=<<B>modules/nixos/tty.nix</B><BR/><BR/><I>1q4j1yybj98hbypqdbnbjag5m1z44zpk</I>> fillcolor="#e3e5b2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.modules" [label=<<B>flake.nix#modules.nixos.modules</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/custom/nixos/mount" [label=<<B>modules/custom/nixos/mount</B><BR/>flake.modules.nixos.modules<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/custom/nixos/wireplumber" [label=<<B>modules/custom/nixos/wireplumber</B><BR/>flake.modules.nixos.modules<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.fonts" [label=<<B>flake.nix#modules.nixos.fonts</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/fonts/nixos.nix" [label=<<B>modules/services/fonts/nixos.nix</B><BR/>flake.modules.nixos.fonts<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.bluetooth" [label=<<B>flake.nix#modules.nixos.bluetooth</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/bluetooth/nixos.nix" [label=<<B>modules/services/bluetooth/nixos.nix</B><BR/>flake.modules.nixos.bluetooth<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.pipewire" [label=<<B>flake.nix#modules.nixos.pipewire</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/pipewire/nixos.nix" [label=<<B>modules/services/pipewire/nixos.nix</B><BR/>flake.modules.nixos.pipewire<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.xdg-settings" [label=<<B>flake.nix#modules.nixos.xdg-settings</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/xdg-settings/nixos.nix" [label=<<B>modules/programs/xdg-settings/nixos.nix</B><BR/>flake.modules.nixos.xdg-settings<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.desktop-utils" [label=<<B>flake.nix#modules.nixos.desktop-utils</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/desktop-utils/kdeconnect.nix" [label=<<B>modules/programs/desktop-utils/kdeconnect.nix</B><BR/>flake.modules.nixos.desktop-utils<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/desktop-utils/udiskie.nix" [label=<<B>modules/programs/desktop-utils/udiskie.nix</B><BR/>flake.modules.nixos.desktop-utils<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.amd" [label=<<B>flake.nix#modules.nixos.amd</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hardware/amd [N].nix" [label=<<B>modules/hardware/amd [N].nix</B><BR/>flake.modules.nixos.amd<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.openssh" [label=<<B>flake.nix#modules.nixos.openssh</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/openssh/nixos.nix" [label=<<B>modules/services/openssh/nixos.nix</B><BR/>flake.modules.nixos.openssh<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.gaming" [label=<<B>flake.nix#modules.nixos.gaming</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/gaming/nixos.nix" [label=<<B>modules/programs/gaming/nixos.nix</B><BR/>flake.modules.nixos.gaming<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.platformOptimizations" [label=<<B>flake.nix#nixosModules.platformOptimizations</B><BR/><BR/><I>2n80z2lpq31ybwfsyb7gg4b95hcia848</I>> fillcolor="#d8b2e5"]
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-modules" [label=<<B>modules</B><BR/>flake.nixosModules.platformOptimizations<BR/><I>2n80z2lpq31ybwfsyb7gg4b95hcia848</I>> fillcolor="#d8b2e5"]
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.wine" [label=<<B>flake.nix#nixosModules.wine</B><BR/><BR/><I>2n80z2lpq31ybwfsyb7gg4b95hcia848</I>> fillcolor="#d8b2e5"]
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.pipewireLowLatency" [label=<<B>flake.nix#nixosModules.pipewireLowLatency</B><BR/><BR/><I>2n80z2lpq31ybwfsyb7gg4b95hcia848</I>> fillcolor="#d8b2e5"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.hyprland" [label=<<B>flake.nix#modules.nixos.hyprland</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/windowManagers/hyprland/nixos.nix" [label=<<B>modules/programs/windowManagers/hyprland/nixos.nix</B><BR/>flake.modules.nixos.hyprland<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.docker" [label=<<B>flake.nix#modules.nixos.docker</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/docker/nixos.nix" [label=<<B>modules/services/docker/nixos.nix</B><BR/>flake.modules.nixos.docker<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.wooting" [label=<<B>flake.nix#modules.nixos.wooting</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/wooting/nixos.nix" [label=<<B>modules/programs/wooting/nixos.nix</B><BR/>flake.modules.nixos.wooting<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"ikdy6g92cm1qc0j9lvsddd478dsd3h6v-nix/module.nix" [label=<<B>nix/module.nix</B><BR/><BR/><I>ikdy6g92cm1qc0j9lvsddd478dsd3h6v</I>> fillcolor="#b2b4e5"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/disk.nix" [label=<<B>modules/hosts/thor/disk.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"zaz0bhdxnm77pvnwn0y07df87b3r97sj-module.nix" [label=<<B>module.nix</B><BR/><BR/><I>zaz0bhdxnm77pvnwn0y07df87b3r97sj</I>> fillcolor="#e5b2de"]
	"zaz0bhdxnm77pvnwn0y07df87b3r97sj-lib/make-disk-image.nix" [label=<<B>lib/make-disk-image.nix</B><BR/><BR/><I>zaz0bhdxnm77pvnwn0y07df87b3r97sj</I>> fillcolor="#e5b2de"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/hardware.nix" [label=<<B>modules/hosts/thor/hardware.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"s08d9f5avsqs69r1y2gyivxgfwv4a88c-nixos/modules/installer/scan/not-detected.nix" [label=<<B>nixos/modules/installer/scan/not-detected.nix</B><BR/><BR/><I>s08d9f5avsqs69r1y2gyivxgfwv4a88c</I>> fillcolor="#e5dbb2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/kopia.nix" [label=<<B>modules/hosts/thor/users/krezh/kopia.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/mounts.nix" [label=<<B>modules/hosts/thor/users/krezh/mounts.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/nixos.nix" [label=<<B>modules/hosts/thor/users/krezh/nixos.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.krezh" [label=<<B>flake.nix#modules.nixos.krezh</B><BR/><BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/users/krezh/nixos.nix" [label=<<B>modules/users/krezh/nixos.nix</B><BR/>flake.modules.nixos.krezh<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/users/krezh/sops/nixos.nix" [label=<<B>modules/users/krezh/sops/nixos.nix</B><BR/>flake.modules.nixos.krezh<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/obsidian.nix" [label=<<B>modules/hosts/thor/users/krezh/obsidian.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/swww.nix" [label=<<B>modules/hosts/thor/users/krezh/swww.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/webapps.nix" [label=<<B>modules/hosts/thor/users/krezh/webapps.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/wlr-which-key.nix" [label=<<B>modules/hosts/thor/users/krezh/wlr-which-key.nix</B><BR/>flake.modules.nixos.thor<BR/><I>faghbcf65xawb76dzcfzd6cp0npnp0k9</I>> fillcolor="#e5dab2"]
	"s08d9f5avsqs69r1y2gyivxgfwv4a88c-flake.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/asmedia.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/disk.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/hardware.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/kopia.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/mounts.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/obsidian.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/swww.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/webapps.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.thor" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/wlr-which-key.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-desktop"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.desktop-utils"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.amd"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.openssh"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.gaming"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.hyprland"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.docker"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.wooting"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/configuration.nix" -> "ikdy6g92cm1qc0j9lvsddd478dsd3h6v-nix/module.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-desktop" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-base"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.fonts"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.bluetooth"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.pipewire"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-desktop/configuration.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.xdg-settings"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.system-base" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "56790dvx2fs9dxp488v886ks40y2pcs0-nixos"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/settings/variables/style.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.shell"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.catppuccin"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/system/system-types/system-base/nixos.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.modules"
	"67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops" -> "67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops/templates"
	"67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops" -> "67n5jl06z5k7zk8rl8zklpk61yrhxwdp-modules/sops/secrets-for-users"
	"56790dvx2fs9dxp488v886ks40y2pcs0-nixos" -> "56790dvx2fs9dxp488v886ks40y2pcs0-nixos/common.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.shell" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/shell/fish.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.catppuccin" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/catppuccin/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/catppuccin/nixos.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-flake.nix#nixosModules.catppuccin"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-flake.nix#nixosModules.catppuccin" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/cursors.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/fcitx5.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/gitea.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/grub.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/gtk.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/limine.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/plymouth.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/sddm.nix"
	"1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/global.nix" -> "1q4j1yybj98hbypqdbnbjag5m1z44zpk-modules/nixos/tty.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.modules" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/custom/nixos/mount"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.modules" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/custom/nixos/wireplumber"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.fonts" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/fonts/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.bluetooth" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/bluetooth/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.pipewire" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/pipewire/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.xdg-settings" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/xdg-settings/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.desktop-utils" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/desktop-utils/kdeconnect.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.desktop-utils" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/desktop-utils/udiskie.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.amd" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hardware/amd [N].nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.openssh" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/openssh/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.gaming" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/gaming/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/gaming/nixos.nix" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.platformOptimizations"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/gaming/nixos.nix" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.wine"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/gaming/nixos.nix" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.pipewireLowLatency"
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.platformOptimizations" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-modules"
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.wine" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-modules"
	"2n80z2lpq31ybwfsyb7gg4b95hcia848-flake.nix#nixosModules.pipewireLowLatency" -> "2n80z2lpq31ybwfsyb7gg4b95hcia848-modules"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.hyprland" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/windowManagers/hyprland/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.docker" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/services/docker/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.wooting" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/programs/wooting/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/disk.nix" -> "zaz0bhdxnm77pvnwn0y07df87b3r97sj-module.nix"
	"zaz0bhdxnm77pvnwn0y07df87b3r97sj-module.nix" -> "zaz0bhdxnm77pvnwn0y07df87b3r97sj-lib/make-disk-image.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/hardware.nix" -> "s08d9f5avsqs69r1y2gyivxgfwv4a88c-nixos/modules/installer/scan/not-detected.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/hosts/thor/users/krezh/nixos.nix" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.krezh"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.krezh" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/users/krezh/nixos.nix"
	"faghbcf65xawb76dzcfzd6cp0npnp0k9-flake.nix#modules.nixos.krezh" -> "faghbcf65xawb76dzcfzd6cp0npnp0k9-modules/users/krezh/sops/nixos.nix"
}
```

</div>

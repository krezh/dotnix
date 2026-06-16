{ inputs, ... }:
{
  flake.images = inputs.self.lib.mkImage {
    name = "installer";
    system = "x86_64-linux";
    stateVersion = "26.11";
  };

  flake.image.installer =
    { pkgs, lib, ... }:
    let
      magic = pkgs.writeShellScriptBin "magic" ''
        set -euo pipefail

        RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'

        FLAKE="github:krezh/dotnix"

        printf "\n''${GREEN}╔══════════════════════════════════════════╗''${NC}\n"
        printf "''${GREEN}║  DotNix NixOS install wizard             ║''${NC}\n"
        printf "''${GREEN}╚══════════════════════════════════════════╝''${NC}\n\n"

        printf "\n''${BLUE}Available hosts:''${NC}\n"
        nix eval "$FLAKE#nixosConfigurations" --apply builtins.attrNames 2>/dev/null \
          | tr -d '[]",' | tr ' ' '\n' | awk 'NF' | sed 's/^/  /' \
          || { printf "''${RED}✗ Failed to list hosts (no network?).''${NC}\n"; exit 1; }
        echo
        read -p "Which host? " HOST
        [ -n "$HOST" ] || { printf "''${RED}Hostname cannot be empty.''${NC}\n"; exit 1; }

        has_internet() {
          curl -sSL --head --max-time 5 https://1.1.1.1 >/dev/null 2>&1 \
            || curl -sSL --head --max-time 5 https://nixos.org >/dev/null 2>&1
        }

        printf "\n''${YELLOW}→ Checking network...''${NC}\n"
        if ! has_internet; then
          sudo systemctl start NetworkManager
          sleep 3
          printf "''${YELLOW}  Opening nmtui...''${NC}\n"
          sudo nmtui
          has_internet || { printf "''${RED}✗ No network. Aborting.''${NC}\n"; exit 1; }
        fi
        printf "''${GREEN}✓ Network OK.''${NC}\n"

        printf "\n''${YELLOW}About to install '$HOST'. This will:''${NC}\n"
        printf "  1. Partition disk (disko)\n"
        printf "  2. Run nixos-install\n"
        printf "  3. Prompt to reboot\n\n"
        read -p "Continue? [y/N] " -n 1 -r
        echo
        [[ "$REPLY" =~ ^[Yy]$ ]] || { printf "''${YELLOW}Aborted.''${NC}\n"; exit 0; }

        sudo disko-install --flake "$FLAKE#$HOST"
      '';
    in
    {
      imports = [
        "${inputs.nixpkgs}/nixos/modules/installer/cd-dvd/installation-cd-minimal.nix"
        inputs.self.modules.nixos.system-base
      ];

      networking.hostName = "dotnix-installer";

      time.timeZone = "Europe/Stockholm";
      console.keyMap = "sv-latin1";

      security.sudo.wheelNeedsPassword = false;

      services.pcscd.enable = true;

      boot.supportedFilesystems = [
        "exfat"
        "ntfs"
        "ext4"
        "btrfs"
        "vfat"
        "xfs"
      ];

      # enable SSH in the boot process
      systemd.services.sshd.wantedBy = pkgs.lib.mkForce [ "multi-user.target" ];

      users.users.nixos.openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
      ];

      services.getty.helpLine = lib.mkForce ''
        ╔══════════════════════════════════════════╗
        ║  DotNix NixOS installer ISO              ║
        ║                                          ║
        ║  Type: magic                             ║
        ║  (interactive install wizard)            ║
        ╚══════════════════════════════════════════╝
      '';

      image.baseName = lib.mkForce "dotnix-installer";
      isoImage.volumeID = lib.mkForce "DOTNIX_INSTALLER";

      environment.systemPackages = [
        pkgs.neovim
        pkgs.gitMinimal
        pkgs.sops
        pkgs.age-plugin-yubikey
        inputs.disko.packages.${pkgs.stdenv.hostPlatform.system}.disko-install
        inputs.disko.packages.${pkgs.stdenv.hostPlatform.system}.disko
        pkgs.nvme-cli
        pkgs.smartmontools
        pkgs.pciutils
        pkgs.usbutils
        pkgs.gptfdisk
        pkgs.iw
        pkgs.tcpdump
        pkgs.wireguard-tools
        pkgs.tmux
        pkgs.btop
        pkgs.rsync
        magic
      ];
    };
}

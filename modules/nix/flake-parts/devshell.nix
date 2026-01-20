{
  perSystem = {
    devshells.default = {
      devshell.startup.a-welcome.text = ''
        echo "❄️ Welcome to the Default shell ❄️"
      '';
      devshell.startup.z-menu.text = "menu";
      devshell = {
        name = "Default";
        motd = "";
        packages = [];
      };
      commands = [
        {
          name = "partition";
          command = ''
            if [ $# -eq 0 ]; then
              echo "Usage: partition <hostname>"
              exit 1
            fi
            sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko -- --mode disko --flake github:krezh/nix-config#"$1"
          '';
          help = "Partition disk for specified host";
          category = "Nix";
        }
        {
          name = "install";
          command = ''
            if [ $# -eq 0 ]; then
              echo "Usage: install <hostname>"
              exit 1
            fi
            sudo nixos-install --flake github:krezh/nix-config#"$1"
          '';
          help = "Install NixOS for specified host";
          category = "Nix";
        }
        {
          name = "disko-install";
          command = ''
            if [ $# -eq 0 ]; then
              echo "Usage: disko-install <hostname>"
              exit 1
            fi
            sudo disko-install --flake github:krezh/nix-config#"$1"
          '';
          help = "Partition and install NixOS in one step";
          category = "Nix";
        }
        {
          name = "wipe-disk";
          command = ''
            if [ $# -eq 0 ]; then
              echo "Usage: wipe-disk <device>"
              echo "Example: wipe-disk /dev/nvme0n1"
              exit 1
            fi
            sudo wipefs -a "$1"
            sudo sgdisk --zap-all "$1"
          '';
          help = "Remove all partitions from specified disk";
          category = "Nix";
        }
      ];
    };
  };
}

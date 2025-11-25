{ ... }:
{
  perSystem =
    {
      config,
      pkgs,
      ...
    }:
    {
      devshells.default = {
        devshell.startup.pre-commit-hook.text = config.pre-commit.installationScript;
        devshell = {
          name = "Default";
          motd = ''
            ❄️ Welcome to the {14}{bold}Default{reset} shell ❄️
          '';
          packages = [
            pkgs.nix-fast-build
          ];
        };
        commands = [
          {
            name = "partition";
            command = ''
              if [ $# -eq 0 ]; then
                echo "Usage: partition <hostname>"
                echo "Available hosts: thor, odin, steamdeck, thor-wsl, nixos-livecd"
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
                echo "Available hosts: thor, odin, steamdeck, thor-wsl, nixos-livecd"
                exit 1
              fi
              sudo nixos-install --flake github:krezh/nix-config#"$1"
            '';
            help = "Install NixOS for specified host";
            category = "Nix";
          }
        ];
      };
    };
}

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
            command = "sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko -- --mode disko --flake github:krezh/nix-config#thor";
            help = "Will partition the disk (Thor)";
            category = "Nix";
          }
          {
            name = "install";
            command = "sudo nixos-install --flake github:krezh/nix-config#thor";
            help = "Will install (Thor)";
            category = "Nix";
          }
        ];
      };
    };
}

{
  config,
  pkgs,
  lib,
  ...
}:
{
  default = {
    devshell.startup.pre-commit-hook.text = config.pre-commit.installationScript;
    devshell = {
      name = "Default";
      motd = ''
        ❄️ Welcome to the {14}{bold}Default{reset} shell ❄️
      '';
      packages = [
        "nix-fast-build"
      ];
    };
    commands = [
      {
        name = "gpa";
        command = "${lib.getExe pkgs.git}/bin/git pull --autostash && ${lib.getExe pkgs.nh} os switch --ask --no-specialisation";
        help = "git pull and os switch";
        category = "Nix";
      }
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
}

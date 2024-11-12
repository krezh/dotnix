{ config, pkgs, ... }:
{
  default = {
    devshell.startup.pre-commit-hook.text = config.pre-commit.installationScript;
    devshell = {
      name = "Default";
      motd = ''
        ❄️ Welcome to the {14}{bold}Default{reset} shell ❄️
      '';
    };
    commands = [
      {
        name = "gpa";
        command = "${pkgs.git}/bin/git pull --autostash && ${pkgs.nh}/bin/nh os switch --ask --no-specialisation";
        help = "git pull and os switch";
        category = "Nix";
      }
      {
        name = "partition";
        command = "sudo nix --experimental-features 'nix-command flakes' run github:nix-community/disko -- --mode disko --flake github:krezh/nix-config#odin";
        help = "Will partition the disk (Odin)";
        category = "Nix";
      }
    ];
  };
}

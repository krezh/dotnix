{ pkgs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    nix-init
  ];

  programs.winbox = {
    enable = true;
    openFirewall = true;
  };
}

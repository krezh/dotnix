{ pkgs, inputs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    nix-init
    inputs.nixd.packages.${pkgs.system}.nixd
    inputs.nix-update.packages.${pkgs.system}.nix-update
  ];

  programs.winbox = {
    enable = true;
    openFirewall = true;
  };

  programs.nh = {
    enable = true;
    package = inputs.nh.packages.${pkgs.system}.default;
  };
}

{ pkgs, inputs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    usbutils
    nix-init
    nix-update
    inputs.nixd.packages.${pkgs.system}.nixd
    nix-inspect
    # inputs.attic.packages.${pkgs.system}.default
  ];

  programs.nh = {
    enable = true;
    package = inputs.nh.packages.${pkgs.system}.default;
  };
}

{ pkgs, ... }:
{
  programs.tofi = {
    enable = true;
    # package = pkgs.tofi;
    catppuccin.enable = true;
  };
}

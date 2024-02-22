{ inputs, pkgs, ... }:
###########################################################
#
# Wezterm Configuration
#
# Useful Hot Keys for Linux(replace `ctrl + shift` with `cmd` on macOS)):
#   1. Increase Font Size: `ctrl + shift + =` | `ctrl + shift + +`
#   2. Decrease Font Size: `ctrl + shift + -` | `ctrl + shift + _`
#   3. And Other common shortcuts such as Copy, Paste, Cursor Move, etc.
#
# Default Keybindings: https://wezfurlong.org/wezterm/config/default-keys.html
#
###########################################################
{

  programs.wezterm = {
      enable = true;

      package = inputs.wezterm.packages.${pkgs.system}.default;

      extraConfig = builtins.readFile ./wezterm.lua;
  };
}
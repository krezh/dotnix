{ pkgs, ... }:
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
  # wezterm has catppuccin theme built-in,
  # it's not necessary to install it separately.

  # we can add wezterm as a flake input once this PR is merged:
  #    https://github.com/wez/wezterm/pull/3547

  programs.wezterm = {
      enable = true;

      # TODO: Fix: https://github.com/wez/wezterm/issues/4483
      # package = pkgs.wezterm.override { };

      extraConfig = builtins.readFile ./wezterm.lua;
  };
}
{
  flake.modules.nixos.xdg-settings = {
    xdg.mime.enable = true;
    xdg.terminal-exec.enable = true;
    xdg.terminal-exec.settings.default = ["kitty"];
  };
}

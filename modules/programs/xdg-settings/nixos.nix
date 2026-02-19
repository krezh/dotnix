{
  flake.modules.nixos.xdg-settings = {
    xdg = {
      mime.enable = true;
      terminal-exec = {
        enable = true;
        settings.default = [ "com.mitchellh.ghostty" ];
      };
    };
  };
}

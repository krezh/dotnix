{ config, pkgs, ... }:
{
  modules.shell.atuin = {
    enable = true;
    package = pkgs.atuin;
    sync_address = "https://sh.talos.plexuz.xyz";
    config = {
      key_path = config.sops.secrets."atuin/key".path;
      style = "compact";
      workspaces = true;
    };
  };
}

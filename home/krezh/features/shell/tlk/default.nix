{ pkgs, ... }:
{
  hmModules.shell.tlk = {
    enable = true;
    package = pkgs.tlk;
    config = {
      proxy = {
        url = "teleport.talos.plexuz.xyz";
        ttl = "1d";
      };
    };
  };
}

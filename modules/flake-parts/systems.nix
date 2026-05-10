{ self, lib, ... }:
let
  ciHosts = lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations;
in
{
  systems = [ "x86_64-linux" ];
  flake = {
    hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) ciHosts;
    ci = {
      hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) ciHosts;
      packages = lib.concatMapAttrs (_system: pkgs: pkgs) self.packages;
    };
  };
}

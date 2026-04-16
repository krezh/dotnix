{ self, lib, ... }:
let
  runnerMap = {
    "x86_64-linux" = "ubuntu-latest";
    "x86_64-darwin" = "ubuntu-latest";
    "aarch64-linux" = "ubuntu-24.04-arm";
  };
  mapToGha = system: runnerMap.${system} or system;
  ciHosts = lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations;
in
{
  systems = [ "x86_64-linux" ];
  flake = {
    hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) ciHosts;
    ghMatrix = {
      include = lib.mapAttrsToList (host: config: {
        inherit host;
        inherit (config.pkgs.stdenv.hostPlatform) system;
        runner = mapToGha config.pkgs.stdenv.hostPlatform.system;
      }) ciHosts;
    };
    ci = {
      hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) ciHosts;
      packages = lib.concatMapAttrs (_system: pkgs: pkgs) self.packages;
    };
  };
}

{
  self,
  lib,
  ...
}: let
  mapToGha = system:
    {
      "x86_64-linux" = "ubuntu-latest";
      "x86_64-darwin" = "ubuntu-latest";
      "aarch64-linux" = "ubuntu-24.04-arm";
    }
    .${
      system
    } or system;
in {
  systems = ["x86_64-linux"];
  flake = {
    hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) (
      lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations
    );

    om.ci.default.root.dir = ".";
    ghMatrix = {
      include = lib.mapAttrsToList (host: config: {
        inherit host;
        system = config.pkgs.stdenv.hostPlatform.system;
        runner = mapToGha config.pkgs.stdenv.hostPlatform.system;
      }) (lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations);
    };
  };
}

{
  inputs,
  self,
  lib,
  ...
}:
let
  inherit (self) outputs;

  flakeLib = import ../lib/flakeLib.nix {
    inherit
      inputs
      outputs
      lib
      self
      ;
  };
  mapToGha =
    system:
    {
      "x86_64-linux" = "ubuntu-latest";
      "x86_64-darwin" = "ubuntu-latest";
      "aarch64-linux" = "ubuntu-24.04-arm";
    }
    .${system} or system;
in
{
  flake = {
    nixosConfigurations = flakeLib.mkSystems {
      thor = {
        system = "x86_64-linux";
        users = [ "krezh" ];
        profiles = [ "gui" ];
      };
      thor-wsl = {
        system = "x86_64-linux";
        users = [ "krezh" ];
        profiles = [ ];
      };
      odin = {
        system = "x86_64-linux";
        users = [ "krezh" ];
        profiles = [ "gui" ];
      };
      steamdeck = {
        system = "x86_64-linux";
        users = [ "krezh" ];
        profiles = [ "gui" ];
        extraModules = [ inputs.jovian.nixosModules.default ];
      };
      buildbot-master = {
        system = "x86_64-linux";
        users = [ ];
        profiles = [ ];
        ci = false;
      };
      buildbot-worker = {
        system = "x86_64-linux";
        users = [ ];
        profiles = [ ];
        ci = false;
      };
    };

    overlays = import ../overlays { inherit inputs lib; };
    homeManagerModules = lib.scanPath.toList { path = ../home/modules; };
    nixosModules.default = lib.scanPath.toImports ../hosts/modules;

    hosts = lib.mapAttrs (_name: config: config.config.system.build.toplevel) (
      lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations
    );

    ghMatrix = {
      include = lib.mapAttrsToList (host: config: {
        inherit host;
        system = config.pkgs.stdenv.hostPlatform.system;
        runner = mapToGha config.pkgs.stdenv.hostPlatform.system;
      }) (lib.filterAttrs (_name: config: (config.ci or true)) self.nixosConfigurations);
    };
  };
}

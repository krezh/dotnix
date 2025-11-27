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
      "x86_64-linux" = "nix-config-runner";
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
        homeUsers = [ "krezh" ];
        profiles = [ "gui" ];
      };
      thor-wsl = {
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
        profiles = [ ];
      };
      odin = {
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
        profiles = [ "gui" ];
      };
      steamdeck = {
        system = "x86_64-linux";
        homeUsers = [ "krezh" ];
        profiles = [ "gui" ];
        extraModules = [ inputs.jovian.nixosModules.default ];
      };
      nixos-livecd = {
        system = "x86_64-linux";
        includeCommon = false;
        ci = false;
      };
    };

    overlays = import ../overlays { inherit inputs lib; };
    homeManagerModules = lib.scanPath.toList { path = ../home/modules; };
    nixosModules.default = lib.scanPath.toImports ../hosts/modules;

    top = lib.mapAttrs (_name: config: config.config.system.build.toplevel) (
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

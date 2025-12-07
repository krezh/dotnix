{
  inputs,
  lib,
  ...
}:
let
  imageConfigs = {
    livecd = {
      format = "install-iso";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "livecd";
        inherit inputs;
      };
      modules = [
        ../images/livecd.nix
      ];
    };

    kubevirt = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
      };
      modules = [
        ../images/kubevirt.nix
      ];
    };

    buildbot-worker = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
      };
      modules = [
        ../images/buildbot-worker.nix
      ];
    };

    buildbot-master = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
      };
      modules = [
        ../images/buildbot-master.nix
      ];
    };
  };
in
{
  flake = {
    images = lib.mapAttrs (_name: config: inputs.nixos-generators.nixosGenerate config) imageConfigs;
  };
}

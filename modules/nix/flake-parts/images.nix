{
  inputs,
  lib,
  ...
}: let
  imageConfigs = {
    livecd = {
      format = "install-iso";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "livecd";
        inherit inputs;
      };
      modules = [
        ../../../images/livecd.nix
      ];
    };

    kubevirt = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
      };
      modules = [
        ../../../images/kubevirt.nix
      ];
    };

    buildbot-worker = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
        inherit inputs;
      };
      modules = [
        inputs.sops-nix.nixosModules.sops
        inputs.self.modules.nixos.buildbot-worker
      ];
    };

    buildbot-master = {
      format = "kubevirt";
      system = "x86_64-linux";
      specialArgs = {
        hostname = "kubevirt";
        inherit inputs;
      };
      modules = [
        inputs.sops-nix.nixosModules.sops
        inputs.self.modules.nixos.buildbot-master
      ];
    };
  };
in {
  flake = {
    images = lib.mapAttrs (_name: config: inputs.nixos-generators.nixosGenerate config) imageConfigs;
  };
}

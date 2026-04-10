{
  flake.modules.nixos.thor-wsl =
    { lib, ... }:
    {
      boot = {
        initrd = {
          availableKernelModules = [ "virtio_pci" ];
          kernelModules = [ ];
        };
        kernelModules = [ "kvm-intel" ];
        extraModulePackages = [ ];
      };

      networking.useDHCP = lib.mkDefault true;
    };
}

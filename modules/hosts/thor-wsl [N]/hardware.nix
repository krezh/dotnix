{
  flake.modules.nixos.thor-wsl = {lib, ...}: {
    boot.initrd.availableKernelModules = ["virtio_pci"];
    boot.initrd.kernelModules = [];
    boot.kernelModules = ["kvm-intel"];
    boot.extraModulePackages = [];

    networking.useDHCP = lib.mkDefault true;
  };
}

{ inputs, ... }:
{
  flake.modules.nixos.phone =
    { modulesPath, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-base
        # inputs.self.modules.nixos.${user}
        (modulesPath + "/profiles/minimal.nix")
      ];
      boot.loader.grub.devices = [ "/boot" ];
    };
}

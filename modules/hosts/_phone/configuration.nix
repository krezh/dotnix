{ inputs, ... }:
{
  flake.modules.nixos.phone =
    { modulesPath, ... }:
    {
      imports = with inputs.self.mods.nixos; [
        system-base
        # inputs.self.mods.nixos.${user}
        (modulesPath + "/profiles/minimal.nix")
      ];
      boot.loader.grub.devices = [ "/boot" ];
    };
}

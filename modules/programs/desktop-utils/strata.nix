{
  flake.modules.nixos.desktop-utils = {
    nixosModules.strata = {
      enable = true;
      backends = {
        smb = true;
        mtp = true;
      };
    };
  };
}

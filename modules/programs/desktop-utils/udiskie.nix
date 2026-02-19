{
  flake.modules = {
    homeManager.desktop-utils = {
      services.udiskie = {
        enable = true;
        automount = true;
        notify = true;
        tray = "never";
      };
    };
    nixos.desktop-utils = {
      services = {
        udisks2.enable = true;
        devmon.enable = true;
        gvfs.enable = true;
      };
    };
  };
}

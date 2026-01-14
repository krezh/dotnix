{
  flake.modules.homeManager.desktop-utils = {
    services.kdeconnect = {
      enable = true;
      indicator = true;
    };
  };
}

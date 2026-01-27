{
  flake.modules.homeManager.television = {
    programs = {
      nix-search-tv = {
        enable = true;
        enableTelevisionIntegration = true;
      };
      television = {
        enable = true;
        enableFishIntegration = true;
        settings = {
          shell_integration = {
            channel_triggers = {
              nix-search-tv = [ "ns" ];
            };
          };
        };
      };
    };

    xdg.desktopEntries = {
      nix-search-tv = {
        name = "Nix Search TV";
        genericName = "Package Search";
        comment = "Search NixOS packages with Television";
        exec = "tv nix-search-tv";
        icon = "system-search";
        terminal = true;
        type = "Application";
        categories = [
          "System"
          "Utility"
        ];
      };
    };
  };
}

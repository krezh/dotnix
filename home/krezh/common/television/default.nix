{
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
    fish = {
      functions = {
        _launch_nix_search = ''
          commandline -f repaint
          tv nix-search-tv
          commandline -f repaint
        '';
      };
      interactiveShellInit = ''
        # Bind CTRL+N to launch nix-search-tv
        bind \cn _launch_nix_search
      '';
    };
  };
}

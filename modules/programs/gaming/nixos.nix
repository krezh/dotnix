{ inputs, ... }:
{
  flake.modules.nixos.gaming =
    { pkgs, ... }:
    {
      imports = [
        inputs.nix-gaming.nixosModules.platformOptimizations
        inputs.nix-gaming.nixosModules.wine
        inputs.nix-gaming.nixosModules.pipewireLowLatency
      ];

      environment = {
        systemPackages = with pkgs; [
          winetricks
          protontricks
          vulkan-tools
          lsfg-vk
          lsfg-vk-ui
          protonplus
          faugus-launcher
        ];
        sessionVariables = {
          STEAM_EXTRA_COMPAT_TOOLS_PATHS = "$HOME/.steam/root/compatibilitytools.d";
        };
      };

      services.lact = {
        enable = true;
      };

      programs = {
        gamescope = {
          enable = true;
          capSysNice = false;
        };
        gamemode = {
          enable = true;
          enableRenice = true;
          settings = {
            custom = {
              start = "${pkgs.libnotify}/bin/notify-send --transient -t 5000 'GameMode' 'Started'";
              end = "${pkgs.libnotify}/bin/notify-send --transient -t 5000 'GameMode' 'Ended'";
            };
          };
        };
        wine = {
          enable = true;
          ntsync = true;
          binfmt = true;
        };
        steam = {
          enable = true;
          package = pkgs.steam.override {
            extraProfile = ''
              unset TZ
            '';
          };
          remotePlay.openFirewall = true;
          localNetworkGameTransfers.openFirewall = true;
          protontricks.enable = true;
          platformOptimizations.enable = true;
        };
      };

      services.pipewire.lowLatency.enable = true;
    };
}

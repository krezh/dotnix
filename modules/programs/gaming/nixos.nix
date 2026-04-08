{ inputs, ... }:
{
  flake.modules.nixos.gaming =
    { pkgs, ... }:
    {
      imports = [
        inputs.nix-gaming.nixosModules.platformOptimizations
        inputs.nix-gaming.nixosModules.wine
        inputs.nix-gaming.nixosModules.pipewireLowLatency
        inputs.steam-config-nix.nixosModules.default
      ];

      environment = {
        sessionVariables = {
          STEAM_EXTRA_COMPAT_TOOLS_PATHS = "$HOME/.steam/root/compatibilitytools.d";
        };
        systemPackages = with pkgs; [
          winetricks
          protontricks
          vulkan-tools
          lsfg-vk
          lsfg-vk-ui
          protonplus
          faugus-launcher
          me3
        ];
      };

      services = {
        pipewire.lowLatency.enable = true;
        lact.enable = true;
      };

      programs = {
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
            extraEnv = {
              MANGOHUD = true;
              MESA_GLSL_CACHE_MAX_SIZE = "16G";
              WINE_CPU_TOPOLOGY = "16:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15";
              PROTON_USE_NTSYNC = true;
              PROTON_USE_WOW64 = 1;
              PROTON_ENABLE_WAYLAND = 1;
              WINE_VK_VULKAN_ONLY = true;
            };
          };
          remotePlay.openFirewall = true;
          localNetworkGameTransfers.openFirewall = true;
          protontricks.enable = true;
          platformOptimizations.enable = true;
          config = {
            enable = true;
            closeSteam = true;
            defaultCompatTool = "Proton-GE Latest";
            apps = {
              eldenring = {
                id = 1245620;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ ];
                  env = {
                    PROTON_USE_WOW64 = 0;
                    PROTON_ENABLE_WAYLAND = 0;
                    LSFG_PROCESS = "Default";
                  };
                };
              };
              witchfire = {
                id = 3156770;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    LSFG_PROCESS = "Default";
                  };
                };
              };
              darktide = {
                id = 1361210;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    PROTON_FSR4_UPGRADE = "4.1.0";
                  };
                };
              };
              fellowship = {
                id = 2352620;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    PROTON_FSR4_UPGRADE = "4.1.0";
                    PROTON_USE_WOW64 = 0;
                    PROTON_ENABLE_WAYLAND = 0;
                  };
                  args = [ "-dx11" ];
                };
              };
              crimson-desert = {
                id = 3321460;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    PROTON_FSR4_UPGRADE = "4.1.0";
                  };
                };
              };
              toxic-commando = {
                id = 2157830;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    PROTON_FSR4_UPGRADE = "4.1.0";
                  };
                };
              };
              sekiro = {
                id = 814380;
                compatTool = "Proton-GE Latest";
                launchOptions = {
                  wrappers = [ "gamemoderun" ];
                  env = {
                    LSFG_PROCESS = "Default";
                  };
                };
              };
            };
          };
        };
      };
    };
}

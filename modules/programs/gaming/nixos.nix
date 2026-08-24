{ inputs, ... }:
{
  flake.modules.nixos.gaming =
    { pkgs, ... }:
    {
      imports = [
        inputs.steam-config-nix.nixosModules.default
      ];

      environment = {
        sessionVariables = {
          STEAM_EXTRA_COMPAT_TOOLS_PATHS = "$HOME/.steam/root/compatibilitytools.d";
        };
        systemPackages = with pkgs; [
          winetricks
          wineWow64Packages.waylandFull
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
        # pipewire.lowLatency.enable = true;
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
        gamescope = {
          enable = true;
          capSysNice = false;
          env = { };
          args = [
            "-W 2560"
            "-H 1440"
            "-r 240"
            "--backend wayland"
            "--expose-wayland"
            "--adaptive-sync"
            "-f"
            "--mangoapp"
          ];
        };
        # wine = {
        # enable = true;
        # ntsync = true;
        # binfmt = true;
        # };
        steam = {
          enable = true;
          package = pkgs.steam.override {
            # extraProfile = ''
            #   unset TZ
            # '';
            extraEnv = {
              MANGOHUD = 1;
              MESA_GLSL_CACHE_MAX_SIZE = "16G";
              WINE_CPU_TOPOLOGY = "16:0,1,2,3,4,5,6,7,16,17,18,19,20,21,22,23"; # Ryzen 9 9950X3D
              PROTON_USE_NTSYNC = 1;
              PROTON_USE_WOW64 = 1;
              PROTON_ENABLE_WAYLAND = 1;
              LOW_LATENCY_LAYER = 1;
              PROTON_DISCORD_BRIDGE = 1;
              MLFG_UPGRADE = 1;
              TZ = "CET-1CEST,M3.5.0,M10.5.0";
              # PROTON_USE_OPTISCALER = 1;
              # PROTON_OPTISCALER_NAME=<name.dll> to control the DLL that should be injected. Three names are supported for PROTON_OPTISCALER_NAME, dxgi.dll, d3d12.dll and dbghelp.dll and defaults to dxgi.dll unless explictly set.
            };
          };
          remotePlay.openFirewall = true;
          localNetworkGameTransfers.openFirewall = true;
          protontricks.enable = true;
          config =
            let
              defaultCompatTool = "Proton-CachyOS Latest";
              gamemode = "${pkgs.gamemode}/bin/gamemoderun";
            in
            {
              enable = true;
              onSteamRunning = "wait";
              inherit defaultCompatTool;
              apps = {
                "1245620" = {
                  name = "eldenring";
                  compatTool = defaultCompatTool;
                  wrappers = [ ];
                  env = {
                    PROTON_USE_WOW64 = 0;
                    PROTON_ENABLE_WAYLAND = 0;
                    LSFG_PROCESS = "Default";
                  };

                };
                "3156770" = {
                  name = "witchfire";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    LSFG_PROCESS = "Default";
                  };

                };
                "2694490" = {
                  name = "poe2";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    # LSFG_PROCESS = "Default";
                  };

                };
                "1361210" = {
                  name = "darktide";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = { };
                };
                "1285190" = {
                  name = "borderlands4";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    PROTON_USE_WOW64 = 0;
                    PROTON_ENABLE_WAYLAND = 0;
                  };
                };
                "2352620" = {
                  name = "fellowship";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    PROTON_USE_WOW64 = 0;
                    PROTON_ENABLE_WAYLAND = 0;
                  };
                  args = [ "-dx11" ];
                };
                "3321460" = {
                  name = "crimson-desert";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = { };
                };
                "881020" = {
                  name = "granblue";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = { };
                };
                "2186680" = {
                  name = "rogue-trader";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    LSFG_PROCESS = "Default";
                  };
                };
                "814380" = {
                  name = "sekiro";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    LSFG_PROCESS = "Default";
                  };
                };
                "3513350" = {
                  name = "wuwa";
                  compatTool = defaultCompatTool;
                  wrappers = [ gamemode ];
                  env = {
                    # LSFG_PROCESS = "Default";
                  };
                };
              };
            };
        };
      };
    };
}

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
            extraProfile = ''
              unset TZ
            '';
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
              # PROTON_USE_OPTISCALER = 1;
              # PROTON_OPTISCALER_NAME=<name.dll> to control the DLL that should be injected. Three names are supported for PROTON_OPTISCALER_NAME, dxgi.dll, d3d12.dll and dbghelp.dll and defaults to dxgi.dll unless explictly set.
            };
          };
          remotePlay.openFirewall = true;
          localNetworkGameTransfers.openFirewall = true;
          protontricks.enable = true;
          # platformOptimizations.enable = true;
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
                eldenring = {
                  id = 1245620;
                  compatTool = defaultCompatTool;
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
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      LSFG_PROCESS = "Default";
                    };
                  };
                };
                poe2 = {
                  id = 2694490;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      # LSFG_PROCESS = "Default";
                    };
                  };
                };
                darktide = {
                  id = 1361210;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = { };
                  };
                };
                borderlands4 = {
                  id = 1285190;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      PROTON_USE_WOW64 = 0;
                      PROTON_ENABLE_WAYLAND = 0;
                    };
                  };
                };
                fellowship = {
                  id = 2352620;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      PROTON_USE_WOW64 = 0;
                      PROTON_ENABLE_WAYLAND = 0;
                    };
                    args = [ "-dx11" ];
                  };
                };
                crimson-desert = {
                  id = 3321460;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = { };
                  };
                };
                granblue = {
                  id = 881020;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = { };
                  };
                };
                rogue-trader = {
                  id = 2186680;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      LSFG_PROCESS = "Default";
                    };
                  };
                };
                sekiro = {
                  id = 814380;
                  compatTool = defaultCompatTool;
                  launchOptions = {
                    wrappers = [ gamemode ];
                    env = {
                      LSFG_PROCESS = "Default";
                    };
                  };
                };
                wuwa = {
                  id = 3513350;
                  compatTool = defaultCompatTool;
                  launchOptions = {
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
    };
}

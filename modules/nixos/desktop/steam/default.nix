{
  config,
  inputs,
  lib,
  pkgs,
  homeUsers,
  ...
}:
let
  cfg = config.nixosModules.desktop.steam;
  defaultSteamCompatToolsPath = "$HOME/.steam/root/compatibilitytools.d";
in
{
  options.nixosModules.desktop.steam = {
    enable = lib.mkEnableOption "steam";
  };

  imports = [
    inputs.nix-gaming.nixosModules.platformOptimizations
    inputs.nix-gaming.nixosModules.wine
    inputs.nix-gaming.nixosModules.pipewireLowLatency
  ];

  config = lib.mkIf cfg.enable {
    environment = {
      systemPackages = with pkgs; [
        winetricks
        vulkan-tools
        sgdboop
        lsfg-vk
        lsfg-vk-ui
        lutris
        protonup
        heroic
        lact
      ];
      sessionVariables = {
        STEAM_EXTRA_COMPAT_TOOLS_PATHS = "${defaultSteamCompatToolsPath}";
      };
    };

    programs = {
      gamescope = {
        enable = true;
        capSysNice = true;
      };
      gamemode = {
        enable = true;
        enableRenice = true;
        settings = {
          custom = {
            start = "${pkgs.libnotify}/bin/notify-send -t 5000 'GameMode' 'Started'";
            end = "${pkgs.libnotify}/bin/notify-send -t 5000 'GameMode' 'Ended'";
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
        extraCompatPackages = [
          # pkgs.proton-ge-bin
        ];
        platformOptimizations.enable = true;
      };
    };

    services.pipewire.lowLatency = {
      enable = false;
    };

    systemd.user.services.steam = {
      enable = true;
      description = "Steam (no-GUI background startup)";
      wantedBy = [ "graphical-session.target" ];
      path = [
        "/run/current-system/sw"
        "/run/wrappers/bin"
      ];
      serviceConfig = {
        ExecStart = "${lib.getExe pkgs.steam} -nochatui -nofriendsui -silent %U";
        Restart = "on-failure";
        RestartSec = "5s";
        Environment = [
          "STEAM_EXTRA_COMPAT_TOOLS_PATHS=${defaultSteamCompatToolsPath}"
        ];
      };
    };

    home-manager.users = lib.genAttrs homeUsers (_user: {
      catppuccin.mangohud.enable = false;
      programs.mangohud = {
        enable = true;
        enableSessionWide = true;
        settings = {
          preset = "0";
          legacy_layout = true;
          round_corners = 10;
          background_alpha = "0.8";
          background_color = "1E1E2E";
          table_columns = 3;
          # Text
          font_size = 26;
          text_color = "CDD6F4";
          text_outline_color = "313244";
          # GPU
          gpu_text = "GPU";
          gpu_stats = true;
          gpu_temp = true;
          gpu_color = "A6E3A1";
          gpu_load_change = true;
          gpu_load_color = "CDD6F4,FAB387,F38BA8";
          # CPU
          cpu_text = "CPU";
          cpu_stats = true;
          cpu_temp = true;
          cpu_color = "89B4FA";
          cpu_load_change = true;
          cpu_load_color = "CDD6F4,FAB387,F38BA8";
          # RAM
          ram = true;
          ram_color = "F5C2E7";
          # Engine
          engine_color = "F38BA8";
          engine_version = true;
          engine_short_names = true;
          # FPS
          fps = true;
          fps_color_change = "F38BA8,F9E2AF,A6E3A1";
          # Wine
          wine = true;
          wine_color = "F38BA8";
          winesync = true;
          # Frame timing
          frame_timing = true;
          frametime_color = "A6E3A1";
          # GameMode
          gamemode = true;
          # FSR
          fsr = true;
          hide_fsr_sharpness = true;
          vulkan_driver = true;
          present_mode = true;
          position = "top-right";
          fps_limit_method = "early";
          toggle_fps_limit = "Shift_L+F3";
          toggle_hud = "Shift_L+F1";
          toggle_preset = "Shift_R+F10";
        };
      };
    });
  };
}

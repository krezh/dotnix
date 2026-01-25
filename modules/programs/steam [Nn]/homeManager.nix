{
  flake.modules.homeManager.steam = {
    catppuccin.mangohud.enable = false;
    programs.mangohud = {
      enable = true;
      enableSessionWide = true;
      settings = {
        preset = 1;
        #   # Default: FPS only (like Steam's FPS counter)
        #   legacy_layout = false;
        #   fps = true;
        #   background_alpha = 0;

        #   # Disable all other stats
        #   gpu_stats = false;
        #   cpu_stats = false;
        #   frame_timing = false;

        #   # Styling
        #   font_size = 26;
        #   text_color = "CDD6F4";
        position = "top-right";
        #   fps_limit_method = "early";

        # Keybinds
        toggle_fps_limit = "Shift_L+F3";
        toggle_hud = "Shift_L+F1";
        toggle_preset = "Shift_R+F10";
      };
    };

    # Create presets.conf to switch to detailed stats
    # xdg.configFile."MangoHud/presets.conf".text = ''
    #   [preset 1]
    #   fps_only=0
    #   legacy_layout=1
    #   round_corners=10
    #   background_alpha=0.8
    #   background_color=1E1E2E
    #   table_columns=3
    #   text_outline_color=313244
    #   gpu_text=GPU
    #   gpu_stats=1
    #   gpu_temp=1
    #   gpu_color=A6E3A1
    #   gpu_load_change=1
    #   gpu_load_color=CDD6F4,FAB387,F38BA8
    #   cpu_text=CPU
    #   cpu_stats=1
    #   cpu_temp=1
    #   cpu_color=89B4FA
    #   cpu_load_change=1
    #   cpu_load_color=CDD6F4,FAB387,F38BA8
    #   ram=1
    #   ram_color=F5C2E7
    #   engine_color=F38BA8
    #   engine_version=1
    #   engine_short_names=1
    #   fps=1
    #   fps_color_change=F38BA8,F9E2AF,A6E3A1
    #   wine=1
    #   wine_color=F38BA8
    #   winesync=1
    #   frame_timing=1
    #   frametime_color=A6E3A1
    #   gamemode=1
    #   fsr=1
    #   hide_fsr_sharpness=1
    #   vulkan_driver=1
    #   present_mode=1
    # '';
  };
}

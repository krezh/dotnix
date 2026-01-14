{
  flake.modules.homeManager.steam = {
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
        font_size = 26;
        text_color = "CDD6F4";
        text_outline_color = "313244";
        gpu_text = "GPU";
        gpu_stats = true;
        gpu_temp = true;
        gpu_color = "A6E3A1";
        gpu_load_change = true;
        gpu_load_color = "CDD6F4,FAB387,F38BA8";
        cpu_text = "CPU";
        cpu_stats = true;
        cpu_temp = true;
        cpu_color = "89B4FA";
        cpu_load_change = true;
        cpu_load_color = "CDD6F4,FAB387,F38BA8";
        ram = true;
        ram_color = "F5C2E7";
        engine_color = "F38BA8";
        engine_version = true;
        engine_short_names = true;
        fps = true;
        fps_color_change = "F38BA8,F9E2AF,A6E3A1";
        wine = true;
        wine_color = "F38BA8";
        winesync = true;
        frame_timing = true;
        frametime_color = "A6E3A1";
        gamemode = true;
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
  };
}

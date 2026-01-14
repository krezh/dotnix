{
  flake.modules.homeManager.terminal =
    { pkgs, config, ... }:
    {
      programs.kitty = {
        enable = true;
        package = pkgs.kitty;
        settings = {
          cursor_trail = 1;
          cursor_shape = "block";
          shell_integration = "no-cursor";
          input_delay = 2;
          font_size = 12;
          font_family = "family=\"${config.var.fonts.mono}\"";
          bold_font = "auto";
          italic_font = "auto";
          bold_italic_font = "auto";
          copy_on_select = "clipboard";
          strip_trailing_spaces = "smart";
          enable_audio_bell = "no";
          sync_to_monitor = "yes";
          scrollback_lines = 10000;
          confirm_os_window_close = 0;
          window_padding_width = 3;
          hide_window_decorations = "yes";
        };
        extraConfig = ''
          mouse_map right click grabbed,ungrabbed no-op
          mouse_map right press grabbed,ungrabbed paste_from_clipboard
        '';
      };
    };
}

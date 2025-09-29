{ pkgs, ... }:
{
  programs.kitty = {
    enable = true;
    package = pkgs.kitty;
    # https://sw.kovidgoyal.net/kitty/conf/
    settings = {
      cursor_trail = 1;
      cursor_shape = "block";
      shell_integration = "no-cursor";
      input_delay = 2;
      font_size = 12;
      font_family = "family=\"JetBrainsMono Nerd Font\"";
      bold_font = "auto";
      italic_font = "auto";
      bold_italic_font = "auto";
      copy_on_select = "clipboard";
      strip_trailing_spaces = "smart";
      enable_audio_bell = "no";
      sync_to_monitor = "yes";
      scrollback_lines = 10000;
    };
    extraConfig = ''
      mouse_map right click grabbed,ungrabbed no-op
      mouse_map right press grabbed,ungrabbed paste_from_clipboard
    '';
  };
}

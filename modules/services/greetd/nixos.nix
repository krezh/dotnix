{
  flake.modules.nixos.greetd =
    { pkgs, ... }:
    {
      services.greetd = {
        enable = true;
        settings = {
          terminal.vt = 1;
          default_session = {
            command = "start-hyprland -- --config /etc/hyprlogin/hyprland-greeter.conf";
            user = "greeter";
          };
        };
      };

      security.pam.services.hyprlogin = { };

      environment = {
        systemPackages = [ pkgs.hyprlogin ];

        etc."hyprlogin/hyprland-greeter.conf".text = ''
          exec-once = ${pkgs.hyprlogin}/bin/hyprlogin --config /etc/hyprlogin/hyprlogin.conf

          monitor = ,preferred,auto,1

          input {
            kb_layout = se
            kb_variant = nodeadkeys
            numlock_by_default = true
          }
        '';

        etc."hyprlogin/hyprlogin.conf".text = ''
          $rosewater = rgb(f5e0dc)
          $flamingo = rgb(f2cdcd)
          $pink = rgb(f5c2e7)
          $mauve = rgb(cba6f7)
          $red = rgb(f38ba8)
          $maroon = rgb(eba0ac)
          $peach = rgb(fab387)
          $yellow = rgb(f9e2af)
          $green = rgb(a6e3a1)
          $teal = rgb(94e2d5)
          $sky = rgb(89dceb)
          $sapphire = rgb(74c7ec)
          $blue = rgb(89b4fa)
          $lavender = rgb(b4befe)
          $text = rgb(cdd6f4)
          $subtext1 = rgb(bac2de)
          $subtext0 = rgb(a6adc8)
          $overlay2 = rgb(9399b2)
          $overlay1 = rgb(7f849c)
          $overlay0 = rgb(6c7086)
          $surface2 = rgb(585b70)
          $surface1 = rgb(45475a)
          $surface0 = rgb(313244)
          $base = rgb(1e1e2e)
          $mantle = rgb(181825)
          $crust = rgb(11111b)

          $accent = $blue
          $font = JetBrainsMono Nerd Font

          sessions {
            wayland_path = /run/current-system/sw/share/wayland-sessions
            x11_path = /run/current-system/sw/share/xsessions
            default_user = krezh
            default_session = hyprland.desktop
          }

          general {
            hide_cursor = true
            immediate_render = true
            exit_command = hyprctl dispatch exit
            no_fade_in = false
            ignore_empty_input = true
          }

          background {
            monitor =
            color = $base
          }

          label {
            monitor =
            text = $LAYOUT
            color = $text
            font_size = 25
            font_family = $font
            onclick = hyprctl switchxkblayout all next
            position = 30, -30
            halign = left
            valign = top
          }

          label {
            monitor =
            text = $TIME
            color = $text
            font_size = 90
            font_family = $font
            position = -30, 0
            halign = right
            valign = top
          }

          label {
            monitor =
            text = cmd[update:43200000] date +"%A, %d %B %Y"
            color = $text
            font_size = 25
            font_family = $font
            position = -30, -150
            halign = right
            valign = top
          }

          label {
            monitor =
            text = $GREETD_SESSION
            font_size = 16
            color = $subtext1
            font_family = $font
            onclick = hyprlogin:session_next
            position = 30, 30
            halign = left
            valign = bottom
          }

          input-field {
            monitor =
            size = 300, 60
            outline_thickness = 4
            rounding = 10
            dots_size = 0.2
            dots_spacing = 0.2
            dots_center = true
            outer_color = $accent
            inner_color = $surface0
            font_color = $text
            fade_on_empty = false
            placeholder_text = <span foreground="##cdd6f4"><i>Password...</i></span>
            placeholder_text_username = <span foreground="##cdd6f4"><i>Username...</i></span>
            hide_input = false
            check_color = $accent
            fail_color = $red
            fail_text = <i>$FAIL <b>($ATTEMPTS)</b></i>
            capslock_color = $yellow
            position = 0, -47
            halign = center
            valign = center
          }
        '';
      };
    };
}

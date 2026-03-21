let
  user = "krezh";
in
{
  flake.modules.nixos.thor =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    {
      home-manager.users.${user} = {
        programs.wlr-which-key = {
          enable = true;
          settings = {
            font = "JetBrainsMono Nerd Font 14";
            background = "#1e1e2e";
            color = "#cdd6f4";
            border = "#89b4fa";
            separator = " ➜ ";
            border_width = 3;
            corner_r = 15;
          };
          menus = {
            browser = [
              {
                key = "h";
                desc = "Helium";
                cmd = lib.getExe pkgs.helium;
              }
              {
                key = "z";
                desc = "Zen";
                cmd = lib.getExe config.home-manager.users.${user}.programs.zen-browser.package;
              }
            ];
            screenshot =
              let
                chompBin = "${lib.getExe pkgs.chomp}";
              in
              [
                {
                  key = "s";
                  desc = "Screen (Fullscreen)";
                  cmd = "${chompBin} --mode image-screen --delay 100";
                }
                {
                  key = "w";
                  desc = "Window";
                  cmd = "${chompBin} --mode image-window --delay 100";
                }
                {
                  key = "a";
                  desc = "Area";
                  cmd = "${chompBin} --mode image-area --delay 100";
                }
                {
                  key = "c";
                  desc = "OCR";
                  cmd = "${chompBin} --ocr";
                }
                {
                  key = "r";
                  desc = "Record";
                  submenu = [
                    {
                      key = "a";
                      desc = "Area";
                      cmd = "${chompBin} --mode video-area --delay 100";
                    }
                    {
                      key = "w";
                      desc = "Window";
                      cmd = "${chompBin} --mode video-window --delay 100";
                    }
                    {
                      key = "s";
                      desc = "Screen";
                      cmd = "${chompBin} --mode video-screen --delay 100";
                    }
                  ];
                }
              ];
          };
        };
      };
    };
}

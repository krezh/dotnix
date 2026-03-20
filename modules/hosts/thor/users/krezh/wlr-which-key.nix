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
                gulpBin = "${lib.getExe pkgs.gulp}";
              in
              [
                {
                  key = "s";
                  desc = "Screen (Fullscreen)";
                  cmd = "${gulpBin} --mode image-screen --delay 100";
                }
                {
                  key = "w";
                  desc = "Window";
                  cmd = "${gulpBin} --mode image-window --delay 100";
                }
                {
                  key = "a";
                  desc = "Area";
                  cmd = "${gulpBin} --mode image-area --delay 100";
                }
                {
                  key = "c";
                  desc = "OCR";
                  cmd = "${gulpBin} --ocr --delay 100";
                }
                {
                  key = "r";
                  desc = "Record";
                  submenu = [
                    {
                      key = "a";
                      desc = "Area";
                      cmd = "${gulpBin} --mode video-area";
                    }
                    {
                      key = "w";
                      desc = "Window";
                      cmd = "${gulpBin} --mode video-window";
                    }
                    {
                      key = "s";
                      desc = "Screen";
                      cmd = "${gulpBin} --mode video-screen --delay 100";
                    }
                  ];
                }
              ];
          };
        };
      };
    };
}

{
  flake.modules.nixos.thor =
    {
      pkgs,
      lib,
      config,
      ...
    }:
    let
      user = "krezh";
    in
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
                  cmd = "${chompBin} --mode image-screen --delay 300";
                }
                {
                  key = "w";
                  desc = "Window";
                  cmd = "${chompBin} --mode image-window --delay 300";
                }
                {
                  key = "a";
                  desc = "Area";
                  cmd = "${chompBin} --mode image-area --delay 300";
                }
                {
                  key = "c";
                  desc = "OCR";
                  cmd = "${chompBin} --ocr --delay 300";
                }
                {
                  key = "r";
                  desc = "Record";
                  submenu = [
                    {
                      key = "a";
                      desc = "Area";
                      cmd = "${chompBin} --mode video-area --delay 300";
                    }
                    {
                      key = "w";
                      desc = "Window";
                      cmd = "${chompBin} --mode video-window --delay 300";
                    }
                    {
                      key = "s";
                      desc = "Screen";
                      cmd = "${chompBin} --mode video-screen --delay 300";
                    }
                  ];
                }
              ];
          };
        };
      };
    };
}

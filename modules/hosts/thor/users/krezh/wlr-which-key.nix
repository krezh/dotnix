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
                recShot = "${lib.getExe pkgs.recshot} -t ${
                  config.home-manager.users.${user}.sops.secrets."zipline/token".path
                } -u https://zipline.talos.plexuz.xyz";
              in
              [
                {
                  key = "s";
                  desc = "Screen (Fullscreen)";
                  cmd = "${recShot} -m image-screen";
                }
                {
                  key = "w";
                  desc = "Window";
                  cmd = "${recShot} -m image-window";
                }
                {
                  key = "a";
                  desc = "Area";
                  cmd = "${recShot} -m image-area";
                }
                {
                  key = "c";
                  desc = "OCR";
                  cmd = "${lib.getExe pkgs.gulp} --ocr --no-snap";
                }
                {
                  key = "r";
                  desc = "Record";
                  submenu = [
                    {
                      key = "a";
                      desc = "Area";
                      cmd = "${recShot} -m video-area";
                    }
                    {
                      key = "w";
                      desc = "Window";
                      cmd = "${recShot} -m video-window";
                    }
                    {
                      key = "s";
                      desc = "Window";
                      cmd = "${recShot} -m video-screen";
                    }
                  ];
                }
              ];
          };
        };
      };
    };
}

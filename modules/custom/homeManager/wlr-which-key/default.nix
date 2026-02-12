{
  flake.modules.homeManager.modules =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.programs.wlr-which-key;
      yamlFormat = pkgs.formats.yaml { };
    in
    {
      options.programs.wlr-which-key = {
        enable = lib.mkEnableOption "wlr-which-key";
        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.wlr-which-key;
          description = "wlr-which-key derivation to use.";
        };
        settings = lib.mkOption {
          type = yamlFormat.type;
          default = { };
          example =
            lib.literalExpression # nix
              ''
                {
                  font = "JetBrainsMono Nerd Font 12";
                  background = "#282828cc";
                  color = "#ebdbb2";
                  border = "#3c3836";
                  border_width = 2;
                  corner_r = 12;
                  padding = 16;
                  separator = " -> ";

                  anchor = "center"; # One of center, left, right, top, bottom, bottom-left, top-left, etc.
                  margin_right = 0; # Only relevant when not center
                  margin_bottom = 0;
                  margin_left = 0;
                  margin_top = 0;
                }
              '';
        };
        menus = lib.mkOption {
          type = lib.types.attrsOf yamlFormat.type;
          default = { };
          example =
            lib.literalExpression # nix
              ''
                {
                  default = [
                    {
                      key = "w";
                      desc = "WiFi";
                      submenu = [
                        {
                          key = "t";
                          desc = "Toggle";
                          cmd = "rfkill toggle wifi";
                        }
                        {
                          key = "c";
                          desc = "Connect";
                          cmd = "kitty -e nmtui-connect";
                        }
                      ];
                    }
                    {
                      key = "l";
                      desc = "Lock";
                      cmd = "pidof swaylock || swaylock";
                    }
                  ];
                }
              '';
        };
      };

      config = lib.mkIf cfg.enable {
        home.packages = [ cfg.package ];
        xdg.configFile = lib.attrsets.mapAttrs' (
          name: menu:
          let
            filename = if name == "default" then "config" else name;
          in
          lib.attrsets.nameValuePair "wlr-which-key/${filename}.yaml" {
            source = (yamlFormat.generate "${filename}.yaml" (cfg.settings // { inherit menu; }));
          }
        ) cfg.menus;
      };
    };
}

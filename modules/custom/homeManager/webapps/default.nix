{
  flake.modules.homeManager.modules =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.programs.webapps;
      desktopDataDir = "${config.xdg.dataHome}/webapps";
      iconDir = "${desktopDataDir}/icons";

      mkApp =
        name: app:
        let
          sanitized = lib.strings.replaceStrings [ " " ] [ "-" ] name;
          iconPath = if app.icon != null then app.icon else "${iconDir}/${sanitized}.png";

          exec =
            "helium "
            + "--app=${app.url} "
            + "--class=${sanitized} "
            + "--user-data-dir=${desktopDataDir}/${sanitized} "
            + "--no-first-run --disable-translate --disable-infobars "
            + "--hide-scrollbars";
        in
        {
          name = sanitized;
          value = {
            inherit name;
            inherit exec;
            icon = iconPath;
            categories = [
              "Network"
              "WebBrowser"
            ];
            type = "Application";
          };
        };

      desktops = lib.attrsets.mapAttrs' mkApp cfg.apps;
    in
    {
      options.programs.webapps = {
        enable = lib.mkEnableOption "dynamic Chromium web app launchers";

        apps = lib.mkOption {
          type = lib.types.attrsOf (
            lib.types.submodule {
              options = {
                url = lib.mkOption {
                  type = lib.types.str;
                  description = "URL of the web app.";
                };
                icon = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = "Path to a custom icon (optional).";
                };
              };
            }
          );
          default = { };
          description = "Web apps to create desktop entries for.";
        };
      };

      config = lib.mkIf cfg.enable {
        xdg.desktopEntries = desktops;

        home.activation.fetchwebappsIcons =
          let
            scriptDeps = with pkgs; [ curl ];
            scriptPath = lib.makeBinPath scriptDeps;
            scriptFile = pkgs.writeShellScript "fetch-webapp-icon" (builtins.readFile ./fetch-icons.sh);

            scriptCalls = lib.concatStringsSep "\n" (
              lib.attrsets.mapAttrsToList (
                name: app:
                let
                  sanitized = lib.strings.replaceStrings [ " " ] [ "-" ] name;
                  generatedIconPath = "${iconDir}/${sanitized}.png";
                in
                if app.icon != null then
                  ""
                else
                  "${lib.escapeShellArg (toString scriptFile)} "
                  + (lib.escapeShellArgs [
                    name
                    app.url
                    generatedIconPath
                  ])
              ) cfg.apps
            );
          in
          lib.hm.dag.entryAfter [ "writeBoundary" ] ''
            export PATH=${lib.escapeShellArg scriptPath}:$PATH
            ${scriptCalls}
          '';
      };
    };
}

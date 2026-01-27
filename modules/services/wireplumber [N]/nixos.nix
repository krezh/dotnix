{
  flake.modules.nixos.wireplumber =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      inherit (lib)
        mkEnableOption
        mkOption
        mkIf
        types
        ;
      cfg = config.nixosModules.wireplumber;

      extractDeviceName =
        nodeName:
        let
          parts = lib.splitString "." nodeName;
        in
        if builtins.length parts >= 2 then builtins.elemAt parts 1 else null;

      mkRule = matches: actions: { inherit matches actions; };
      mkMatch = condition: [ condition ];
      mkUpdateProps = props: { update-props = props; };

      groupByDevice =
        items: extractFn:
        builtins.foldl' (
          acc: item:
          let
            device = extractDeviceName (extractFn item);
          in
          if device != null then acc // { ${device} = (acc.${device} or [ ]) ++ [ item ]; } else acc
        ) { } items;

      hideNodesByDevice = groupByDevice cfg.hideNodes (x: x);
      renameModulesByDevice = groupByDevice cfg.renameModules (x: x.nodeName);
      allDevices = builtins.attrNames (hideNodesByDevice // renameModulesByDevice);

      mkHideRules =
        nodeNames:
        map (
          nodeName:
          mkRule (mkMatch { "node.name" = nodeName; }) (mkUpdateProps {
            "node.disabled" = "true";
          })
        ) nodeNames;

      mkRenameRules =
        modules:
        map (
          {
            nodeName,
            description,
            nick,
          }:
          mkRule (mkMatch { "node.name" = nodeName; }) (mkUpdateProps {
            "node.description" = description;
            "node.nick" = nick;
          })
        ) modules;

      generateDeviceConfig =
        device:
        let
          settings =
            cfg.deviceSettings.${device} or {
              priority = 50;
              deviceProps = { };
            };
          deviceHideNodes = hideNodesByDevice.${device} or [ ];
          deviceRenameModules = renameModulesByDevice.${device} or [ ];
          configName = "${toString settings.priority}-${device}-config";
          deviceCardRule =
            if settings.deviceProps != { } then
              [
                (mkRule (mkMatch { "device.name" = "alsa_card.${device}"; }) (mkUpdateProps settings.deviceProps))
              ]
            else
              [ ];
          hideRules = mkHideRules deviceHideNodes;
          renameRules = mkRenameRules deviceRenameModules;
        in
        {
          name = configName;
          value = {
            "monitor.alsa.rules" = deviceCardRule ++ hideRules ++ renameRules;
          };
        };

      audio-switch-script = pkgs.writeShellScript "audio-switch" (
        lib.replaceStrings
          [ "wpctl" "@PRIMARY_DEVICE_NAME@" "@SECONDARY_DEVICE_NAME@" ]
          [ "${pkgs.wireplumber}/bin/wpctl" cfg.audioSwitching.primary cfg.audioSwitching.secondary ]
          (builtins.readFile ./audio-switch.sh)
      );
    in
    {
      options.nixosModules.wireplumber = {
        enable = mkEnableOption "custom audio device configuration";
        audioSwitching = {
          enable = mkEnableOption "manual audio device switching";
          primary = mkOption {
            type = types.str;
            default = "A50 Game Audio";
            description = "Display name of the primary audio device";
          };
          secondary = mkOption {
            type = types.str;
            default = "Argon Speakers";
            description = "Display name of the secondary audio device";
          };
        };
        hideNodes = mkOption {
          type = types.listOf types.str;
          default = [ ];
          description = "List of ALSA node names to hide";
        };
        renameModules = mkOption {
          type = types.listOf (
            types.submodule {
              options = {
                nodeName = mkOption {
                  type = types.str;
                  description = "ALSA node name to rename";
                };
                description = mkOption {
                  type = types.str;
                  description = "New description for the node";
                };
                nick = mkOption {
                  type = types.str;
                  description = "New nickname for the node";
                };
              };
            }
          );
          default = [ ];
          description = "List of nodes to rename";
        };
        deviceSettings = mkOption {
          type = types.attrsOf (
            types.submodule {
              options = {
                priority = mkOption {
                  type = types.int;
                  default = 50;
                  description = "Priority for configuration loading";
                };
                deviceProps = mkOption {
                  type = types.attrs;
                  default = { };
                  description = "Device-specific properties";
                };
              };
            }
          );
          default = { };
          description = "Device-specific settings by device name";
        };
      };

      config = mkIf cfg.enable {
        services.pipewire.wireplumber.extraConfig = builtins.listToAttrs (
          map generateDeviceConfig allDevices
        );
        environment.systemPackages = lib.optionals cfg.audioSwitching.enable [
          (pkgs.writeShellScriptBin "audio-switch" (builtins.readFile audio-switch-script))
        ];
        systemd.user.services.wireplumber.restartTriggers = [
          (builtins.toJSON (builtins.listToAttrs (map generateDeviceConfig allDevices)))
        ];
        systemd.user.services.pipewire.restartTriggers = [
          (builtins.toJSON (builtins.listToAttrs (map generateDeviceConfig allDevices)))
        ];
      };
    };
}

{
  flake.modules.nixos.modules =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.nixosModules.strata;
      gvfsPackage =
        (pkgs.gnome.gvfs.override {
          samba = if cfg.backends.smb then pkgs.samba else null;
          libimobiledevice = if cfg.backends.afc then pkgs.libimobiledevice else null;
          libgphoto2 = if cfg.backends.gphoto2 then pkgs.libgphoto2 else null;
          libmtp = if cfg.backends.mtp then pkgs.libmtp else null;
        }).overrideAttrs
          (oldAttrs: {
            mesonFlags = (oldAttrs.mesonFlags or [ ]) ++ [
              "-Dafc=${lib.boolToString cfg.backends.afc}"
              "-Dgphoto2=${lib.boolToString cfg.backends.gphoto2}"
              "-Dmtp=${lib.boolToString cfg.backends.mtp}"
              "-Dsmb=${lib.boolToString cfg.backends.smb}"
            ];
          });
    in
    {
      options.nixosModules.strata = {
        enable = lib.mkEnableOption "Strata runtime integration";

        package = lib.mkOption {
          type = lib.types.package;
          default = pkgs.strata;
          description = "Strata package to install system-wide.";
        };

        backends = {
          smb = lib.mkEnableOption "SMB access through GVfs";
          afc = lib.mkEnableOption "iPhone and iPad app-document access through GVfs AFC";
          gphoto2 = lib.mkEnableOption "camera and PTP photo access through GVfs";
          mtp = lib.mkEnableOption "Android file access through GVfs MTP";
        };
      };

      config = lib.mkIf cfg.enable {
        environment.systemPackages = [ cfg.package ];
        services.gvfs = {
          enable = true;
          package = gvfsPackage;
        };

        services.usbmuxd.enable = cfg.backends.afc;
      };
    };
}

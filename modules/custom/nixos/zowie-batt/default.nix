{
  flake.modules.nixos.modules =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      inherit (lib)
        mkEnableOption
        mkIf
        mkPackageOption
        ;
      cfg = config.nixosModules.zowie-batt;
    in
    {
      options.nixosModules.zowie-batt = {
        enable = mkEnableOption "zowie-batt — battery readout for BenQ ZOWIE DW wireless mice";
        package = mkPackageOption pkgs "zowie-batt" { };
      };

      config = mkIf cfg.enable {
        environment.systemPackages = [ cfg.package ];
        services.udev.packages = [ cfg.package ];
      };
    };
}

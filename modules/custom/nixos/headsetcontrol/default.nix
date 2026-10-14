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
      cfg = config.nixosModules.headsetcontrol;
    in
    {
      options.nixosModules.headsetcontrol = {
        enable = mkEnableOption "HeadsetControl — control USB headsets (sidetone, LEDs, equalizer, battery)";
        package = mkPackageOption pkgs "headsetcontrol" { };
      };

      config = mkIf cfg.enable {
        environment.systemPackages = [ cfg.package ];
        services.udev.packages = [ cfg.package ];
      };
    };
}

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
      cfg = config.programs.headsetcontrol;
    in
    {
      options.programs.headsetcontrol = {
        enable = mkEnableOption "HeadsetControl — control USB headsets (sidetone, LEDs, equalizer, battery)";
        package = mkPackageOption pkgs "headsetcontrol" { };
      };

      config = mkIf cfg.enable {
        environment.systemPackages = [ cfg.package ];
        services.udev.packages = [ cfg.package ];
      };
    };
}
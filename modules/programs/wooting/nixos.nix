{
  flake.modules.nixos.wooting =
    { pkgs, ... }:
    {
      environment = {
        systemPackages = with pkgs; [
          wootility
        ];
      };
      services.udev.packages = [ pkgs.wooting-udev-rules ];
    };
}

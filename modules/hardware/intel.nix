{
  flake.modules.nixos.intel =
    { pkgs, ... }:
    {
      # Intel GPU
      hardware = {
        graphics = {
          enable = true;
          extraPackages = with pkgs; [
            intel-media-driver
            libvdpau-va-gl
          ];
        };
      };
      environment = {
        sessionVariables = {
          LIBVA_DRIVER_NAME = "iHD";
        };
        systemPackages = with pkgs; [
          intel-gpu-tools
        ];
      };
    };
}

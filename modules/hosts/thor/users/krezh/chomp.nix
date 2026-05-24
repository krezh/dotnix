{
  flake.modules.nixos.thor =
    { config, pkgs, ... }:
    let
      user = "krezh";
    in
    {
      home-manager.users.${user} = {
        programs.chomp = {
          enable = true;
          font.family = config.var.fonts.sans;
          border = {
            thickness = 3;
            rounding = 15;
          };
          annotate.package = pkgs.satty;
          zipline = {
            url = "https://zipline.plexuz.xyz";
            token = config.home-manager.users.${user}.sops.secrets."zipline/token".path;
            useOriginalName = true;
          };
        };
      };
    };
}

{
  flake.modules.nixos.thor =
    { config, ... }:
    let
      user = "krezh";
    in
    {
      home-manager.users.${user} = {
        homeModules.chomp = {
          enable = true;
          font.family = config.var.fonts.sans;
          border = {
            thickness = config.var.borderSize;
            rounding = config.var.rounding;
          };
          zipline = {
            url = "https://zipline.plexuz.xyz";
            token = config.home-manager.users.${user}.sops.secrets."zipline/token".path;
            useOriginalName = true;
          };
          capture.replay.hyprlandTag = "games";
        };
      };
    };
}

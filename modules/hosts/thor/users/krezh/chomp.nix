{
  flake.modules.nixos.thor =
    { config, ... }:
    let
      user = "krezh";
    in
    {
      home-manager.users.${user} = {
        programs.chomp = {
          enable = true;
          font.family = "Rubik";
          border = {
            thickness = 3;
            rounding = 15;
          };
          zipline = {
            url = "https://zipline.talos.plexuz.xyz";
            token = config.home-manager.users.${user}.sops.secrets."zipline/token".path;
            useOriginalName = true;
          };
        };
      };
    };
}

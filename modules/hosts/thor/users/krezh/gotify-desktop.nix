{
  flake.modules.nixos.thor =
    let
      user = "krezh";
    in
    {
      home-manager.users.${user} =
        { config, ... }:
        {
          sops.secrets."gotify-desktop/token" = { };
          services.gotify-desktop = {
            enable = true;
            tokenFile = config.sops.secrets."gotify-desktop/token".path;
            url = "wss://gotify.plexuz.xyz";
          };
        };
    };
}

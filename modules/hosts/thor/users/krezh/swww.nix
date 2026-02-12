let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    home-manager.users.${user} = {
      services.swww-random = {
        enable = true;
        settings.interval = 60 * 10; # 10 minutes
      };
    };
  };
}

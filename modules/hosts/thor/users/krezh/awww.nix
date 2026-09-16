let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    home-manager.users.${user} = {
      homeModules.awww-random = {
        enable = true;
        settings.interval = 60 * 10; # 10 minutes
      };
    };
  };
}

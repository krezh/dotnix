let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    home-manager.users.${user} = {
      programs.obsidian = {
        enable = true;
        vaults.plexuz.target = "Obsidian/Plexuz";
      };
    };
  };
}

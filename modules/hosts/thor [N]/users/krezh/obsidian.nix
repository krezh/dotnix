{
  flake.modules.nixos.thor = {
    home-manager.users.krezh = {
      programs.obsidian = {
        enable = true;
        vaults.plexuz.target = "Obsidian/Plexuz";
      };
    };
  };
}

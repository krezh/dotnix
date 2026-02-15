{ inputs, ... }:
{
  flake.modules.homeManager.system-base =
    {
      lib,
      config,
      osConfig,
      ...
    }:
    {
      imports = [
        inputs.sops-nix.homeManagerModules.sops
        inputs.nix-index.homeModules.nix-index
      ]
      ++ (with inputs.self.modules; [
        generic.var
        homeManager.shell
        homeManager.catppuccin
        homeManager.modules
      ]);

      home = {
        homeDirectory = lib.mkDefault "/home/${config.home.username}";
        stateVersion = osConfig.system.stateVersion;
        preferXdgDirectories = true;
      };

      manual.manpages.enable = false;

      programs = {
        home-manager.enable = true;
      };

      systemd.user.startServices = "sd-switch";
    };
}

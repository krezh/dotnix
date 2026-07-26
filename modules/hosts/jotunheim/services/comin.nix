{ inputs, ... }:
{
  flake.modules.nixos.jotunheim = {
    imports = [ inputs.comin.nixosModules.comin ];

    services.comin = {
      enable = true;
      remotes = [
        {
          name = "origin";
          url = "https://github.com/krezh/dotnix.git";
          branches.main.name = "main";
        }
      ];
    };
  };
}

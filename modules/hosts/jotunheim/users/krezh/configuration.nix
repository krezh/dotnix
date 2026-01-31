{ inputs, ... }:
let
  user = "krezh";
in
{
  flake.modules.nixos.jotunheim = {

    home-manager.users.${user} = {
      imports = with inputs.self.modules.homeManager; [
        system-base
      ];
    };
  };
}

{
  inputs,
  outputs,
  lib,
  ...
}:
let
  # function to make `pkgs` for defined system with my overlays
  mkPkgsWithSystem =
    system:
    import inputs.nixpkgs {
      inherit system;
      overlays = builtins.attrValues outputs.overlays;
      config = {
        allowUnfree = true;
        allowUnfreePredicate = _: true;
      };
    };
in
{
  mkSystem =
    {
      hostname,
      system ? "x86_64-linux",
      # homeUsers is list of home-manager users for the host
      # each user needs a `./home-manager/<name>/default.nix` file present
      # set it to empty list to disable home-manager altogether for the host
      homeUsers ? [ ],
      # baseModules is the base of the entire machine building
      baseModules ? [ ],
      # extraModules is additional modules you want to add for the host
      extraModules ? [
        inputs.sops-nix.nixosModules.sops
        inputs.home-manager.nixosModules.home-manager
        inputs.catppuccin.nixosModules.catppuccin
      ],
    }:
    let
      mkHomeUsers =
        users:
        if users == [ ] then
          users
        else
          [
            {
              config.home-manager = {
                users = builtins.listToAttrs (
                  builtins.map (name: {
                    name = name;
                    value = {
                      imports = [ ./home/${name} ];
                      config.home = {
                        username = name;
                      };
                    };
                  }) users
                );
                backupFileExtension = "bk";
                useGlobalPkgs = true;
                useUserPackages = true;
                extraSpecialArgs = {
                  inherit inputs outputs hostname;
                };
                sharedModules = [
                  inputs.sops-nix.homeManagerModules.sops
                  inputs.catppuccin.homeManagerModules.catppuccin
                ];
              };
            }
          ];
    in
    lib.nixosSystem {
      pkgs = mkPkgsWithSystem system;
      specialArgs = {
        inherit
          inputs
          outputs
          hostname
          lib
          ;
      };
      modules = baseModules ++ extraModules ++ mkHomeUsers homeUsers;
    };

  # mkHome =
  #   {
  #     hostname,
  #     username ? "krezh",
  #     system ? "x86_64-linux",
  #   }:
  #   inputs.home-manager.lib.homeManagerConfiguration {
  #     pkgs = mkPkgsWithSystem system;
  #     extraSpecialArgs = {
  #       inherit
  #         inputs
  #         outputs
  #         hostname
  #         lib
  #         ;
  #     };
  #     modules = [
  #       inputs.sops-nix.homeManagerModules.sops
  #       {
  #         imports = [ ./home/${username} ];
  #         config.home.username = username;
  #       }
  #     ];
  #   };
}

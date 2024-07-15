{
  inputs,
  lib,
  pkgs,
  config,
  ...
}:
{
  imports = [ inputs.lix-module.nixosModules.default ];

  nix = {
    package = pkgs.nixFlakes;
    extraOptions = lib.optionalString (config.nix.package == pkgs.nixFlakes) ''
      !include ${config.sops.templates."nix_access_token.conf".path}
      experimental-features = nix-command flakes
    '';
    settings = {
      trusted-users = [
        "root"
        "@wheel"
      ];
      auto-optimise-store = lib.mkDefault true;
      experimental-features = [
        "nix-command"
        "flakes"
        "repl-flake"
      ];
      warn-dirty = false;
      system-features = [
        "kvm"
        "big-parallel"
      ];
      flake-registry = ""; # Disable global flake registry
      use-xdg-base-directories = true;
    };
    gc = {
      automatic = true;
      dates = "weekly";
    };

    channel.enable = true;

    # Add each flake input as a registry
    # To make nix3 commands consistent with the flake
    registry = lib.mapAttrs (_: value: { flake = value; }) inputs;

    # Add nixpkgs input to NIX_PATH
    # This lets nix2 commands still use <nixpkgs>
    nixPath = [ "nixpkgs=${inputs.nixpkgs.outPath}" ];
  };
}

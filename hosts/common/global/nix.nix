{
  inputs,
  lib,
  config,
  pkgs,
  ...
}:
{
  nix = {
    package = pkgs.lixPackageSets.stable.lix;
    extraOptions = ''
      !include ${config.sops.templates."nix_access_token.conf".path}
    '';
    settings = {
      keep-outputs = true;
      keep-derivations = true;
      warn-dirty = false;
      flake-registry = ""; # Disable global flake registry
      use-xdg-base-directories = true;
      trusted-users = [
        "root"
        "@wheel"
      ];
      auto-optimise-store = true;
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      system-features = [
        "kvm"
        "big-parallel"
      ];
    };
    gc = {
      automatic = true;
      dates = "weekly";
    };

    channel.enable = false;

    # Add each flake input as a registry
    # To make nix3 commands consistent with the flake
    registry = lib.mapAttrs (_: value: { flake = value; }) inputs;

    nixPath = lib.mkForce [ "nixpkgs=${inputs.nixpkgs}" ];
  };
}

{
  inputs,
  lib,
  config,
  pkgs,
  ...
}:
{
  imports = [ ];
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
      accept-flake-config = true;
      always-allow-substitutes = true;
      builders-use-substitutes = true;
      trusted-users = [ "@wheel" ];
      auto-optimise-store = true;
      experimental-features = [
        "nix-command"
        "flakes"
      ];
      system-features = [ ];
      extra-substituters = [
        "https://nix-cache.plexuz.xyz/krezh"
        "https://nix-community.cachix.org"
      ];
      extra-trusted-public-keys = [
        "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      ];
    };
    gc = {
      automatic = true;
      dates = lib.mkDefault "weekly";
    };

    channel.enable = lib.mkForce false;

    # Add each flake input as a registry
    # To make nix3 commands consistent with the flake
    registry = lib.mapAttrs (_: value: { flake = value; }) (
      lib.filterAttrs (name: _: name != "self") inputs
    );

    nixPath = lib.mkForce [ "nixpkgs=${inputs.nixpkgs}" ];
  };
  system.stateVersion = lib.mkDefault "24.05";
}

{
  inputs,
  lib,
  config,
  ...
}:
{
  imports = [ inputs.determinate.nixosModules.default ];
  nix = {
    # package = pkgs.lixPackageSets.stable.lix;
    extraOptions = ''
      !include ${config.sops.templates."nix_access_token.conf".path}
    '';
    settings = {
      eval-cores = 0;
      keep-outputs = true;
      keep-derivations = true;
      warn-dirty = false;
      flake-registry = ""; # Disable global flake registry
      use-xdg-base-directories = true;
      accept-flake-config = true;
      always-allow-substitutes = true;
      builders-use-substitutes = true;
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
      extra-substituters = [
        "https://nix-cache.plexuz.xyz/krezh"
        "https://nix-gaming.cachix.org"
        "https://cache.garnix.io"
        "https://krezh.cachix.org"
        "https://nix-community.cachix.org"
        "https://hyprland.cachix.org"
      ];
      extra-trusted-public-keys = [
        "krezh:bCYQVVbREhrYgC42zUMf99dMtVXIATXMCcq+wRimqCc="
        "nix-gaming.cachix.org-1:nbjlureqMbRAxR1gJ/f3hxemL9svXaZF/Ees8vCUUs4="
        "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g="
        "krezh.cachix.org-1:0hGx8u/mABpZkzJEBh/UMXyNon5LAXdCRqEeVn5mff8="
        "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
        "hyprland.cachix.org-1:a7pgxzMz7+chwVL3/pzj6jIBMioiJM7ypFP8PwtkuGc="
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
  system.stateVersion = lib.mkDefault "24.05";
}

{ inputs, ... }:
{
  flake.modules.nixos.system-base =
    {
      lib,
      pkgs,
      config,
      ...
    }:
    {
      nixpkgs = {
        config.allowUnfree = true;
        overlays = [ inputs.self.overlays.default ];
      };

      system.stateVersion = lib.mkDefault "24.05";

      i18n = {
        defaultLocale = lib.mkDefault "en_US.UTF-8";
        extraLocales = "all";
        extraLocaleSettings.LC_TIME = "en_US.UTF-8";
      };

      nix = {
        package = lib.mkDefault pkgs.lixPackageSets.latest.lix;
        extraOptions = ''
          !include ${config.sops.templates."nix_access_token.conf".path}
        '';
        settings = {
          keep-outputs = lib.mkDefault false;
          keep-derivations = lib.mkDefault false;
          warn-dirty = false;
          flake-registry = "";
          use-xdg-base-directories = true;
          accept-flake-config = true;
          always-allow-substitutes = true;
          builders-use-substitutes = true;
          use-cgroups = true;
          auto-optimise-store = true;
          log-format = "multiline-with-logs";
          warn-import-from-derivation = true;
          trusted-users = [ "@wheel" ];
          fallback = true;
          http-connections = 0;
          experimental-features = [
            "nix-command"
            "flakes"
            "cgroups"
          ];
          extra-substituters = [
            # "https://nix-cache.plexuz.xyz/krezh"
            "https://niks.plexuz.xyz"
            "https://nix-community.cachix.org"
          ];
          extra-trusted-public-keys = [
            # "krezh:GBrZyWDPWYTg/9a9Vad/NRQF/1w0Yc1kWXOQXM3d0RQ="
            "niks.plexuz.xyz-1:dBHlH3p4D7VL2bEW3csdOtJ/X3HTWiCUapJfua48DMg="
            "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
          ];
        };
        gc = {
          automatic = true;
          dates = lib.mkDefault "weekly";
        };
        channel.enable = lib.mkForce false;
        registry = lib.mapAttrs (_: value: { flake = value; }) (
          lib.filterAttrs (name: _: name != "self") inputs
        );
        nixPath = lib.mkForce [ "nixpkgs=${inputs.nixpkgs}" ];
      };

      time.timeZone = "Europe/Stockholm";

      environment.variables = {
        TZ = config.time.timeZone;
      };

      documentation = {
        enable = lib.mkForce false;
        man = {
          enable = lib.mkForce false;
          cache.enable = lib.mkForce false;
        };
      };
    };
}

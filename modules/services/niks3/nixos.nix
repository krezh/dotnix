{ inputs, ... }: {
  flake.modules.nixos.niks3 = { config, ... }: {
    imports = [ inputs.niks3.nixosModules.niks3-auto-upload ];
    services.niks3-auto-upload = {
      enable = true;
      serverUrl = "https://niks.plexuz.xyz";
      authTokenFile = config.sops.secrets."niks3/token".path;
    };
  };
}

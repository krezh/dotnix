{inputs, ...}: {
  flake.modules.homeManager.browsers = {pkgs, ...}: {
    home.packages = [inputs.helium.packages.${pkgs.stdenv.hostPlatform.system}.helium];
  };
}

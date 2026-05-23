{
  flake.modules.homeManager.editors =
    { lib, pkgs, ... }:
    {
      vscodium.extensionIds = [ "opentofu.vscode-opentofu" ];

      programs.vscodium.profiles.default.userSettings = {
        opentofu = {
          codelens.referenceCount = true;
          experimentalFeatures.prefillRequiredFields = true;
          languageServer = {
            path = lib.getExe pkgs.tofu-ls;
            tofu.path = lib.getExe pkgs.opentofu;
          };
        };
        "[opentofu]".editor.defaultFormatter = "opentofu.vscode-opentofu";
      };
    };
}

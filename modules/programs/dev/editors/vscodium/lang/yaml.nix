{
  flake.modules.homeManager.editors = { pkgs, lib, ... }: {
    vscodium.extensionIds = [
      "redhat.vscode-yaml"
    ];

    programs.vscodium.profiles.default.extensions = [ pkgs.yayamlls-vscode ];

    programs.vscodium.profiles.default.userSettings = {
      yaml = {
        format.enable = true;
        validate = true;
      };
      yayamlls.path = lib.getExe pkgs.yayamlls;
      "[yaml]" = {
        editor = {
          defaultFormatter = "esbenp.prettier-vscode";
        };
        diffEditor.ignoreTrimWhitespace = true;
      };
    };
  };
}

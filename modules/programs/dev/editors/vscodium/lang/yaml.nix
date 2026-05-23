{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [
      "redhat.vscode-yaml"
    ];

    programs.vscodium.profiles.default.userSettings = {
      yaml = {
        format.enable = true;
        validate = true;
      };
      "[yaml]" = {
        editor = {
          defaultFormatter = "esbenp.prettier-vscode";
        };
        diffEditor.ignoreTrimWhitespace = true;
      };
    };
  };
}

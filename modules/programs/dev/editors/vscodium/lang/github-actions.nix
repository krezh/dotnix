{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "github.vscode-github-actions" ];

    programs.vscodium.profiles.default.userSettings = {
      "[github-actions-workflow]".editor.defaultFormatter = "esbenp.prettier-vscode";
    };
  };
}

{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [
      "github.vscode-github-actions"
      "zizmor.zizmor-vscode"
    ];

    programs.vscodium.profiles.default.userSettings = {
      "[github-actions-workflow]".editor.defaultFormatter = "esbenp.prettier-vscode";
    };
  };
}

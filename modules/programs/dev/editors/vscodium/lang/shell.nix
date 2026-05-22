{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [
      "timonwong.shellcheck"
      "jeff-hykin.better-shellscript-syntax"
      "mads-hartmann.bash-ide-vscode"
    ];

    programs.vscodium.profiles.default.userSettings = {
      "[shellscript]".editor.defaultFormatter = "mads-hartmann.bash-ide-vscode";
      "[fish]".editor.defaultFormatter = "bmalehorn.vscode-fish";
    };
  };
}

{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "nefrob.vscode-just-syntax" ];

    programs.vscodium.profiles.default.userSettings = {
      "[just]".editor.defaultFormatter = "nefrob.vscode-just-syntax";
    };
  };
}

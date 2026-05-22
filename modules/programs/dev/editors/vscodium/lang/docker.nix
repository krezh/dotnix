{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "docker.docker" ];

    programs.vscodium.profiles.default.userSettings = {
      docker.extension.enableComposeLanguageServer = false;
      "[dockerbake]".editor.defaultFormatter = "docker.docker";
      "[dockercompose]".editor.defaultFormatter = "esbenp.prettier-vscode";
    };
  };
}

{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "tamasfe.even-better-toml" ];

    programs.vscodium.profiles.default.userSettings = {
      "[toml]".editor.defaultFormatter = "tamasfe.even-better-toml";
    };
  };
}

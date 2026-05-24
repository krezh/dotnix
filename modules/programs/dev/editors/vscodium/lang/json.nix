{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "blueglassblock.better-json5" ];

    programs.vscodium.profiles.default.userSettings = {
      "[json]".editor.defaultFormatter = "vscode.json-language-features";
      "[jsonc]".editor = {
        quickSuggestions.strings = true;
        suggest.insertMode = "replace";
      };
    };
  };
}

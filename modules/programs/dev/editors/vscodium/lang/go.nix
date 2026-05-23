{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [ "golang.go" ];

    programs.vscodium.profiles.default.userSettings = {
      go = {
        toolsManagement.autoUpdate = true;
        inlayHints.assignVariableTypes = true;
      };
      gopls."ui.documentation.hoverKind" = "FullDocumentation";
      "[go]".editor.defaultFormatter = "golang.go";
    };
  };
}

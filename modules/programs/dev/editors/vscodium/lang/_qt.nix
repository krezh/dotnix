{
  flake.modules.homeManager.editors =
    { pkgs, ... }:
    {
      vscodium.extensionIds = [
        "theqtcompany.qt-core"
        "theqtcompany.qt-qml"
        "theqtcompany.qt-ui"
      ];

      programs.vscodium.profiles.default.userSettings = {
        workbench = {
          editorAssociations = {
            "*.qrc" = "qt-core.qrcEditor";
            "{git,gitlens,chat-editing-snapshot-text-model,git-graph,git-graph-3}:/**/*.qrc" = "default";
            "{git,gitlens,chat-editing-snapshot-text-model,git-graph,git-graph-3}:/**/*.ui" = "default";
          };
        };
        "[qml]".editor.defaultFormatter = "theqtcompany.qt-qml";
        qt-core.additionalQtPaths = [
          {
            name = "Qt6-nix";
            path = "${pkgs.qt6.qtbase}/bin/qtpaths";
          }
        ];
        qt-qml = {
          doNotAskForQmllsDownload = true;
          qmlls = {
            useQmlImportPathEnvVar = true;
            customExePath = "${pkgs.qt6.qtdeclarative}/bin/qmlls";
          };
        };
      };
    };
}

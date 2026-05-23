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

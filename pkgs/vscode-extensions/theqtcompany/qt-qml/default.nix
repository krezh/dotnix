{
  lib,
  vscode-utils,
}:

vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = {
    name = "qt-qml";
    publisher = "theqtcompany";
    # renovate: datasource=custom.open-vsix depName=theqtcompany/qt-qml
    version = "1.11.1";
    hash = "sha256-lUXx2VAXK0Av4T3bRW7hXpP0u7zJbDvMbKkpPACT4WE=";
  };

  meta = {
    description = "Qt QML Support for VS Code";
    license = lib.licenses.mit;
  };
}

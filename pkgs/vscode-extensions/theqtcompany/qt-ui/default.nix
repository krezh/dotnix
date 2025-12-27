{
  lib,
  vscode-utils,
}:

vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = {
    name = "qt-ui";
    publisher = "theqtcompany";
    # renovate: datasource=custom.open-vsix depName=theqtcompany/qt-ui
    version = "1.11.1";
    hash = "sha256-LScVryXxPTqTqDt8Hx2jWUmKflRskajeXMwJmdUtE4E=";
  };

  meta = {
    description = "Qt UI Support for VS Code";
    license = lib.licenses.mit;
  };
}

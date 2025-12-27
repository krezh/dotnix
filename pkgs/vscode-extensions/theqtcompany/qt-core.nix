{
  lib,
  vscode-utils,
}:

vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = {
    name = "qt-core";
    publisher = "theqtcompany";
    # renovate: datasource=custom.open-vsix depName=theqtcompany/qt-core
    version = "1.11.1";
    hash = "sha256-PQmNWezNYVGGNFAJrlMRhXHe3o0XV6LxE2omU1mqZM0=";
  };

  meta = {
    description = "Qt Core Support for VS Code";
    license = lib.licenses.mit;
  };
}

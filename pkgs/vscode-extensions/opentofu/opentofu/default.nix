{
  vscode-utils,
}:

vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = {
    name = "vscode-opentofu";
    publisher = "OpenTofu";
    # renovate: datasource=custom.open-vsix depName=OpenTofu/vscode-opentofu
    version = "0.6.0";
    hash = "sha256-BXzR1jmifawIIwA0RxnqVOGrpT5/gHV4lPIcYfqAaeM=";
  };

  meta = {
    description = "OpenTofu Support for VS Code";
  };
}

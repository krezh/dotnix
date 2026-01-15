{
  vscode-utils,
}:

vscode-utils.buildVscodeMarketplaceExtension {
  mktplcRef = {
    name = "better-json5";
    publisher = "BlueGlassBlock";
    # renovate: datasource=custom.open-vsix depName=BlueGlassBlock/better-json5
    version = "1.6.0";
    hash = "sha256-ySGU7LZqymZBfsKaVwKrqrIMGEItBMea5LM+/DHABFM=";
  };

  meta = {
    description = "JSON5 Support for VS Code";
  };
}

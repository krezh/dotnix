_final: prev: {
  vscode-extensions = prev.vscode-extensions // {
    theqtcompany = prev.callPackage ../pkgs/vscode-extensions/theqtcompany { };
  };
}

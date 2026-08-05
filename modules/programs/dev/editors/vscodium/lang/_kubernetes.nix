{
  flake.modules.homeManager.editors = {
    vscodium.extensionIds = [
      "helm-ls.helm-ls"
      "ms-kubernetes-tools.vscode-kubernetes-tools"
    ];

    programs.vscodium.profiles.default.userSettings = {
      vs-kubernetes."vs-kubernetes.crd-code-completion" = "disabled";
    };
  };
}

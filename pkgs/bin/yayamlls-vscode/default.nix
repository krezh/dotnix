{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  pkgs,
}:

buildNpmPackage (finalAttrs: {
  pname = "yayamlls-vscode";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.1.10";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-E0loH//MdaXui/tLoiNoL+L1RBPgh0TlnF1M6flxKto=";
  };

  sourceRoot = "source/editors/vscode";

  npmDepsFetcherVersion = 2;
  npmDepsHash = "sha256-n9ll6fYJoh6LV5ibWoVwHg2rU5DbEE1aivdyCRY8EzE=";

  nativeBuildInputs = [
    pkgs.vsce
    pkgs.unzip
    pkgs.jq
  ];

  postPatch = ''
    ${lib.getExe pkgs.jq} '.engines.vscode = .devDependencies["@types/vscode"]' \
      package.json > package.json.tmp
    mv package.json.tmp package.json
  '';

  buildPhase = "vsce package";

  installPhase = ''
    unzip -q yayamlls-*.vsix -d unpacked
    mkdir -p "$out/share/vscode/extensions/home-operations.yayamlls"
    cp -r unpacked/extension/. "$out/share/vscode/extensions/home-operations.yayamlls/"
  '';

  passthru = {
    vscodeExtPublisher = "home-operations";
    vscodeExtName = "yayamlls";
    vscodeExtUniqueId = "home-operations.yayamlls";
  };

  meta = {
    description = "YAML language server VS Code extension";
    homepage = "https://github.com/home-operations/yayamlls";
    license = lib.licenses.mit;
  };
})

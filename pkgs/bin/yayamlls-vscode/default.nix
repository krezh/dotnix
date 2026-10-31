{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  pkgs,
}:

buildNpmPackage (finalAttrs: {
  pname = "yayamlls-vscode";
  # renovate: datasource=github-releases depName=home-operations/yayamlls
  version = "0.3.3";

  src = fetchFromGitHub {
    owner = "home-operations";
    repo = "yayamlls";
    tag = finalAttrs.version;
    hash = "sha256-dX7FtnsARxr4K89eGcclRxdISVcHHcR8Wr6g6YHcvUM=";
  };

  sourceRoot = "source/editors/vscode";

  npmDepsFetcherVersion = 2;
  npmDepsHash = "sha256-wkxgGIR//t9aHRlYE7X4PcUFLvt70P7x4HgMh0PCIj4=";

  nativeBuildInputs = [
    pkgs.vsce
    pkgs.unzip
    pkgs.jq
  ];

  buildPhase = "vsce package";

  installPhase = ''
    unzip -q yayamlls-*.vsix -d unpacked
    ${lib.getExe pkgs.jq} --arg ver "^${lib.versions.major pkgs.vscodium.version}.${lib.versions.minor pkgs.vscodium.version}.0" \
      '.engines.vscode = $ver' \
      unpacked/extension/package.json > unpacked/extension/package.json.tmp
    mv unpacked/extension/package.json.tmp unpacked/extension/package.json
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
    changelog = "https://github.com/home-operations/yayamlls/releases/tag/${finalAttrs.version}";
    license = lib.licenses.mit;
  };
})

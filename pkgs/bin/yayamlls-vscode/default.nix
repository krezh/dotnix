{
  lib,
  buildNpmPackage,
  fetchFromGitHub,
  vscodium,
  pkgs,
}:

buildNpmPackage (
  finalAttrs:
  let
    repoSrc = fetchFromGitHub {
      owner = "home-operations";
      repo = "yayamlls";
      tag = finalAttrs.version;
      hash = "sha256-j0Tke9jK/ZEvCZoAoharRMN0wkhBPXhYcm4uJ6Ogi7o=";
    };
  in
  {
    pname = "yayamlls-vscode";
    # renovate: datasource=github-releases depName=home-operations/yayamlls
    version = "0.1.3";

    src = "${repoSrc}/editors/vscode";

    npmDepsHash = "sha256-+v19xyyflJ+RS60+zUHyukrQXliI+zfZMIj5cYrHHn8=";

    nativeBuildInputs = [
      pkgs.vsce
      pkgs.unzip
      pkgs.jq
    ];

    postPatch = ''
      ${lib.getExe pkgs.jq} '.engines.vscode = "^${lib.versions.major vscodium.version}.${lib.versions.minor vscodium.version}.0"' \
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
  }
)

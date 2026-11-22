{
  lib,
  pkgs,
}:
let
  pname = "kopiur";
  # renovate: datasource=github-releases depName=home-operations/kopiur
  version = "0.11.2";

  selectSystem =
    attrs:
    attrs.${pkgs.stdenvNoCC.hostPlatform.system}
      or (throw "${pname}: unsupported system ${pkgs.stdenvNoCC.hostPlatform.system}");

  suffix = selectSystem {
    x86_64-linux = "linux_amd64";
    aarch64-linux = "linux_arm64";
    x86_64-darwin = "darwin_amd64";
    aarch64-darwin = "darwin_arm64";
  };

  hash = selectSystem {
    x86_64-linux = "sha256-eFqoCbUZE8hLJStdgr2M3sWL1688R8WUb2KyRCHx1bA=";
    aarch64-linux = "sha256-d7HE3mHWmAb7Md7hzQ8wItokKUFGIAlH57R8PjHUWjk=";
    x86_64-darwin = "sha256-MA799tgJpMaEwgvZVRHVPKu5rYOgoeYA8+gdeHy7wGI=";
    aarch64-darwin = "sha256-ISsxEvzez1F/+aYAOcTS/fajRUM4JB4CqBBSA8sp2dg=";
  };
in
pkgs.stdenvNoCC.mkDerivation (_finalAttrs: {
  inherit pname version;

  src = pkgs.fetchurl {
    url = "https://github.com/home-operations/kopiur/releases/download/${version}/kubectl-kopiur_${version}_${suffix}.tar.gz";
    inherit hash;
  };

  sourceRoot = ".";

  installPhase = ''
    runHook preInstall
    install -Dm755 kopiur $out/bin/kopiur
    ln -s $out/bin/kopiur $out/bin/kubectl-kopiur
    runHook postInstall
  '';

  meta = {
    description = "Kubectl plugin operating the kopiur Kopia-native backup operator";
    homepage = "https://kopiur.home-operations.com/cli/";
    changelog = "https://github.com/home-operations/kopiur/blob/${version}/CHANGELOG.md";
    license = lib.licenses.agpl3Only;
    mainProgram = "kopiur";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
  };
})

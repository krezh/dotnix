{
  lib,
  pkgs,
}:

pkgs.python3Packages.buildPythonApplication (
  finalAttrs:
  let
    pythonDeps = with pkgs.python3Packages; [
      dbus-python
      icoextract
      pillow
      psutil
      pygobject3
      requests
      vdf
    ];
  in
  {
    pname = "faugus-launcher";
    # renovate: datasource=github-releases depName=Faugus/faugus-launcher
    version = "2.5.7";
    pyproject = false;

    src = pkgs.fetchFromGitHub {
      owner = "Faugus";
      repo = "faugus-launcher";
      tag = finalAttrs.version;
      hash = "sha256-lB7/X7ilnoIc27Ggr+xCL/B0Gb9fTHWJNE9ajfYPccI=";
    };

    nativeBuildInputs = [
      pkgs.gobject-introspection
      pkgs.meson
      pkgs.ninja
      pkgs.wrapGAppsHook4
    ];

    buildInputs = [
      pkgs.gtk4
      pkgs.libadwaita
      pkgs.libgudev
      pkgs.libmanette
    ];

    dependencies = pythonDeps;

    postPatch = ''
      substituteInPlace faugus-launcher \
        --replace-fail "/usr/bin/python3" "${pkgs.python3Packages.python.interpreter}"

      substituteInPlace faugus/path_manager.py \
        --replace-fail "PathManager.user_data('faugus-launcher/umu-run')" "'${lib.getExe pkgs.umu-launcher}'" \
        --replace-fail 'next((p for p in _LSFGVK_CANDIDATES if p.exists()), _LSFGVK_CANDIDATES[-1])' 'Path("${pkgs.lsfg-vk}/lib/liblsfg-vk.so")'
    '';

    preFixup = ''
      gappsWrapperArgs+=(
        --prefix PYTHONPATH : "$out/${pkgs.python3Packages.python.sitePackages}:${pkgs.python3Packages.makePythonPath pythonDeps}"
        --suffix PATH : "${
          lib.makeBinPath [
            pkgs.python3Packages.icoextract
            pkgs.imagemagick
            pkgs.umu-launcher
            pkgs.xdg-utils
          ]
        }"
      )
    '';

    meta = {
      description = "Simple and lightweight app for running Windows games using UMU-Launcher";
      homepage = "https://github.com/Faugus/faugus-launcher";
      changelog = "https://github.com/Faugus/faugus-launcher/releases/tag/${finalAttrs.src.tag}";
      license = lib.licenses.mit;
      mainProgram = "faugus-launcher";
      platforms = lib.platforms.linux;
    };
  }
)

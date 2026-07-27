{
  fetchFromGitHub,
  gobject-introspection,
  gtk4,
  imagemagick,
  lib,
  libadwaita,
  libgudev,
  libmanette,
  meson,
  ninja,
  python3Packages,
  umu-launcher,
  lsfg-vk,
  wrapGAppsHook4,
  xdg-utils,
}:

python3Packages.buildPythonApplication (finalAttrs:
  let
    pythonDeps = with python3Packages; [
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
    version = "2.0.3";
    pyproject = false;

    src = fetchFromGitHub {
      owner = "Faugus";
      repo = "faugus-launcher";
      tag = finalAttrs.version;
      hash = "sha256-Eqf6grEy7LUITRVpTN9x0UJB1QYc7ZrNlAjCYE8RFHA=";
    };

    nativeBuildInputs = [
      gobject-introspection
      meson
      ninja
      wrapGAppsHook4
    ];

    buildInputs = [
      gtk4
      libadwaita
      libgudev
      libmanette
    ];

    dependencies = pythonDeps;

    postPatch = ''
      substituteInPlace faugus-launcher \
        --replace-fail "/usr/bin/python3" "${python3Packages.python.interpreter}"

      substituteInPlace faugus/path_manager.py \
        --replace-fail "PathManager.user_data('faugus-launcher/umu-run')" "'${lib.getExe umu-launcher}'" \
        --replace-fail 'next((p for p in _LSFGVK_CANDIDATES if p.exists()), _LSFGVK_CANDIDATES[-1])' 'Path("${lsfg-vk}/lib/liblsfg-vk.so")'
    '';

    preFixup = ''
      gappsWrapperArgs+=(
        --prefix PYTHONPATH : "$out/${python3Packages.python.sitePackages}:${python3Packages.makePythonPath pythonDeps}"
        --suffix PATH : "${lib.makeBinPath [ python3Packages.icoextract imagemagick umu-launcher xdg-utils ]}"
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
})

{
  fetchFromGitHub,
  gobject-introspection,
  icoextract,
  imagemagick,
  lib,
  libayatana-appindicator,
  libcanberra-gtk3,
  meson,
  ninja,
  python3Packages,
  umu-launcher,
  lsfg-vk,
  wrapGAppsHook3,
  xdg-utils,
}:

python3Packages.buildPythonApplication (finalAttrs: {
  pname = "faugus-launcher";
  # renovate: datasource=github-releases depName=Faugus/faugus-launcher
  version = "1.22.7";
  pyproject = false;

  src = fetchFromGitHub {
    owner = "Faugus";
    repo = "faugus-launcher";
    tag = finalAttrs.version;
    hash = "sha256-Ki1Me/mh8zniCpjjFVl1Dv/svJ0ZgDKVOG1vmhoLxIo=";
  };

  nativeBuildInputs = [
    gobject-introspection
    meson
    ninja
    wrapGAppsHook3
  ];

  buildInputs = [
    libayatana-appindicator
  ];

  dependencies = with python3Packages; [
    pillow
    psutil
    pygobject3
    requests
    vdf
  ];

  postPatch = ''
    substituteInPlace faugus-launcher \
      --replace-fail "/usr/bin/python3" "${python3Packages.python.interpreter}"

    substituteInPlace faugus/path_manager.py \
      --replace-fail "PathManager.user_data('faugus-launcher/umu-run')" "'${lib.getExe umu-launcher}'"

    substituteInPlace faugus/path_manager.py \
      --replace-fail 'next((p for p in _lsfgvk_candidates if p.exists()), _lsfgvk_candidates[-1])' 'Path("${lsfg-vk}/lib/liblsfg-vk.so")'
  '';

  dontWrapGApps = true;

  preFixup =
    let
      pythonPath =
        with python3Packages;
        makePythonPath [
          pillow
          psutil
          pygobject3
          requests
          vdf
        ];
    in
    ''
      # Wrap faugus-launcher manually
      gappsWrapperArgs+=(
        --prefix PYTHONPATH : "$out/${python3Packages.python.sitePackages}:${pythonPath}"
        --suffix PATH : "${
          lib.makeBinPath [
            icoextract
            imagemagick
            libcanberra-gtk3
            umu-launcher
            xdg-utils
          ]
        }"
      )
      wrapProgram $out/bin/faugus-launcher "''${gappsWrapperArgs[@]}"

      # Set wrapper args for faugus-run
      makeWrapperArgs+=(
        "''${gappsWrapperArgs[@]}"
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

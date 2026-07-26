{
  lib,
  stdenv,
  cmake,
  fetchFromGitHub,
  pkg-config,
  hidapi,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "headsetcontrol";
  # renovate: datasource=github-releases depName=Sapd/HeadsetControl
  version = "4.0.0";

  src = fetchFromGitHub {
    owner = "Sapd";
    repo = "HeadsetControl";
    tag = finalAttrs.version;
    hash = "sha256-3mwX6XqoW5wphBHEqDQ1LMCSCv+3OtNlE9cz4M437ME=";
  };

  nativeBuildInputs = [
    cmake
    pkg-config
  ];

  buildInputs = [ hidapi ];

  cmakeFlags = [ "-DCMAKE_BUILD_TYPE=Release" ];

  # Upstream's CMakeLists.txt derives the version from `git describe`. The
  # fetched tarball has no .git, so inject the real version via a stub git.
  postPatch = ''
    cat > git <<'EOF'
    #!${stdenv.shell}
    case "$1 $2" in
      "describe --tags") echo "${finalAttrs.version}" ;;
      "rev-parse --short") echo "nix" ;;
      *) exit 1 ;;
    esac
    EOF
    chmod +x git
    export PATH=$PWD:$PATH
  '';

  meta = {
    description = "Tool to control USB-connected headsets (sidetone, LEDs, equalizer, battery, etc.)";
    homepage = "https://github.com/Sapd/HeadsetControl";
    changelog = "https://github.com/Sapd/HeadsetControl/releases/tag/${finalAttrs.version}";
    license = lib.licenses.gpl3Plus;
    mainProgram = "headsetcontrol";
    platforms = lib.platforms.linux ++ lib.platforms.darwin ++ lib.platforms.windows;
  };
})
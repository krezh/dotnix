{ pkgs }:
pkgs.stdenv.mkDerivation (finalAttrs: {
  pname = "ps4-pkg-tools";
  version = "20250825-123142-e7c40358";

  src = pkgs.fetchFromGitHub {
    owner = "xXJSONDeruloXx";
    repo = "ps4-pkg-tools";
    rev = "v${finalAttrs.version}";
    hash = "sha256-t4vs2OEAssSY28XwQngNGv8wui1VKjlYLDpj3JZL/Fg=";
    fetchSubmodules = true;
  };

  nativeBuildInputs = [
    pkgs.cmake
    pkgs.pkg-config
    pkgs.qt6.wrapQtAppsHook
    pkgs.wrapGAppsHook3
  ];

  buildInputs = [
    pkgs.qt6.qtbase
    pkgs.gsettings-desktop-schemas
  ];

  postInstall = ''
        mkdir -p $out/share/applications
        cat > $out/share/applications/ps4-pkg-tool-gui.desktop << EOF
    [Desktop Entry]
    Name=PS4 PKG Tool
    Exec=ps4-pkg-tool-gui
    Terminal=false
    Type=Application
    Icon=application-x-executable
    Comment=Extract and decrypt PlayStation 4 PKG files
    Categories=Utility;
    EOF
  '';

  meta = {
    description = "Lightweight command-line utility and GUI application for extracting and decrypting PlayStation 4 PKG files";
    homepage = "https://github.com/xXJSONDeruloXx/ps4-pkg-tools";
    license = pkgs.lib.licenses.gpl2Plus;
    maintainers = with pkgs.lib.maintainers; [ ];
    mainProgram = "ps4-pkg-tool";
    platforms = pkgs.lib.platforms.linux;
  };
})

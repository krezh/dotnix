{
  lib,
  craneLib,
  pkgs,
  makeWrapper,
}:
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;

  buildInputs = with pkgs; [
    wayland
    wayland-protocols
    wayland-scanner
    libxkbcommon
  ];

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src strictDeps buildInputs;
    nativeBuildInputs = with pkgs; [
      pkg-config
    ];
  };

  nativeBuildInputs = with pkgs; [
    pkg-config
    makeWrapper
  ];

  postInstall = ''
    wrapProgram $out/bin/slumber \
      --prefix PATH : ${lib.makeBinPath [ pkgs.fontconfig ]}
  '';

  meta = {
    description = "Fast, reactive Wayland/Hyprland screensaver with Catppuccin themes and animations";
    platforms = lib.platforms.linux;
    mainProgram = "slumber";
  };
}

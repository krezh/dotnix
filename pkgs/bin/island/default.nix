{
  lib,
  craneLib,
  pkgs,
}:
let
  runtimeLibs = with pkgs; [
    vulkan-loader
    wayland
    libxkbcommon
  ];
  src = craneLib.cleanCargoSource ./.;
in
craneLib.buildPackage rec {
  inherit src;
  strictDeps = true;

  cargoVendorDir = craneLib.vendorCargoDeps {
    inherit src;
    overrideVendorCargoPackage =
      p: drv:
      if lib.elem p.name [
        "gpui-pre"
        "gpui-pre-linux"
      ] then
        drv.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [ ./patches/${p.name}-set-layer.patch ];
        })
      else
        drv;
  };

  ISLAND_PCI_IDS = "${pkgs.hwdata}/share/hwdata/pci.ids";

  buildInputs = with pkgs; [
    fontconfig
    freetype
    libpulseaudio
    libxkbcommon
    udev
    wayland
  ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit
      src
      strictDeps
      cargoVendorDir
      buildInputs
      nativeBuildInputs
      ;
  };

  postFixup = ''
    patchelf --add-rpath ${lib.makeLibraryPath runtimeLibs} $out/bin/island
  '';

  meta = {
    description = "Dynamic island status bar and notification daemon built with GPUI";
    platforms = lib.platforms.linux;
    mainProgram = "island";
  };
}

{
  craneLib,
  pkg-config,
  pkgs,
  wrapGAppsHook4,
}:
craneLib.buildPackage rec {
  src = craneLib.cleanCargoSource ./.;
  strictDeps = true;
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src strictDeps;
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [
      pkgs.gtk4
      pkgs.libadwaita
      pkgs.openssl
    ];
  };

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    pkgs.gtk4
    pkgs.libadwaita
    pkgs.openssl
  ];

  meta = {
    description = "GTK4 manager for OptiScaler upscaling mod for Steam games";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
    ];
    mainProgram = "optiman";
  };
}

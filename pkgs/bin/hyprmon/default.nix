{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "hyprmon";
  # renovate: datasource=github-releases depName=erans/hyprmon
  version = "0.0.8";

  src = fetchFromGitHub {
    owner = "erans";
    repo = "hyprmon";
    rev = "v${version}";
    hash = "sha256-woe2FzFKGBhh65eCBplKEpks4BsBVdt35P31HqosxN8=";
  };

  vendorHash = "sha256-GawRPqgG0vqIDZig837g5bEU94Dv03lU2vTeSs5xx5E=";

  ldflags = [
    "-s"
    "-w"
  ];

  meta = {
    description = "TUI monitor configuration tool for Hyprland with visual layout, drag-and-drop, and profile management";
    homepage = "https://github.com/erans/hyprmon/";
    license = lib.licenses.asl20;
    mainProgram = "hyprmon";
  };
}

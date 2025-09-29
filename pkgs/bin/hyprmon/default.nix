{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "hyprmon";
  # renovate: datasource=github-releases depName=erans/hyprmon
  version = "0.0.10";

  src = fetchFromGitHub {
    owner = "erans";
    repo = "hyprmon";
    rev = "v${version}";
    hash = "";
  };

  vendorHash = "";

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

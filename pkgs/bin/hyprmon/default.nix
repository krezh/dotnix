{
  lib,
  buildGoModule,
  fetchFromGitHub,
  go,
}:
buildGoModule rec {
  pname = "hyprmon";
  # renovate: datasource=github-releases depName=erans/hyprmon
  version = "0.0.12";
  src = fetchFromGitHub {
    owner = "erans";
    repo = "hyprmon";
    rev = "v${version}";
    hash = "sha256-jZUtdOMmpd75CyjaXdrqXcYxcQ9q7G2YGBHoUUvycX8=";
  };
  vendorHash = "sha256-THfdsr8jSvbcV1C2C2IJNvjeeonSZDfmCo6Ws2WreBA=";

  # Automatically patch go.mod to use the available Go version
  postPatch = ''
    GO_VERSION=$(${go}/bin/go version | grep -oP 'go\K[0-9]+\.[0-9]+(\.[0-9]+)?')
    sed -i "s|^go [0-9]\+\.[0-9]\+.*|go $GO_VERSION|" go.mod
  '';

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

{
  pkgs,
  lib,
  buildGoApplication,
  go-bin,
}:
let
  htmx = pkgs.fetchurl {
    url = "https://unpkg.com/htmx.org@2.0.4/dist/htmx.min.js";
    hash = "sha256-4gndpcgjVHnzFm3vx3UOHbzVpcGAi3eS/C5nM3aPtEc=";
  };
in
buildGoApplication rec {
  pname = "nixos-webgui";
  version = "0.1.0";

  src = builtins.path {
    path = ./.;
    name = "nixos-webgui-src";
  };

  go = go-bin.latestStable;
  modules = "${src}/govendor.toml";

  nativeBuildInputs = [
    pkgs.nodejs
    pkgs.tailwindcss
  ];

  preBuild = ''
    # Vendor htmx — fetched from nix store, no network at build time
    mkdir -p ./static/js
    cp ${htmx} ./static/js/htmx.min.js

    # Build Tailwind CSS
    echo "Building Tailwind CSS..."
    tailwindcss -i ./static/css/input.css -o ./static/css/output.css --minify
  '';

  # Embed static assets
  tags = [ "embed" ];

  ldflags = [
    "-s"
    "-w"
  ];

  meta = with lib; {
    description = "A modern web GUI for managing NixOS systems";
    homepage = "https://github.com/krezh/nix-config";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "nixos-webgui";
  };
}

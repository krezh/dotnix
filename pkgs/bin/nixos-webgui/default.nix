{
  pkgs,
  lib,
  buildGoApplication,
  go-bin,
}:
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
    pkgs.templ
    pkgs.nodejs
    pkgs.tailwindcss
  ];

  preBuild = ''
    # Generate Go code from Templ templates
    echo "Generating templates..."
    templ generate

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

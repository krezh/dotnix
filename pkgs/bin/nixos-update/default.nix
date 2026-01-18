{
  lib,
  buildGoApplication,
}:

buildGoApplication {
  pname = "nixos-update";
  version = "0.1.0";
  src = builtins.path {
    path = ./.;
    name = "nixos-update-src";
  };
  modules = ./gomod2nix.toml;
  ldflags = [
    "-s"
    "-w"
  ];

  postInstall = ''
    mv $out/bin/nixos-update $out/bin/nu
  '';

  meta = with lib; {
    description = "A stylish NixOS system updater using nh os switch";
    homepage = "https://github.com/krezh/nix-config";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "nu";
  };
}

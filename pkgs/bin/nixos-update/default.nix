{
  lib,
  buildGoApplication,
}:

buildGoApplication {
  pname = "nixos-update";
  version = "0.1.0";
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./go.mod
      ./go.sum
      ./gomod2nix.toml
      ./main.go
    ];
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

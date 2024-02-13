{
  description = "REST API for any Postgres database";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
  };

  outputs = {self, nixpkgs}: {
    defaultPackage.x86_64-linux =
      with import nixpkgs { system = "x86_64-linux"; };

      stdenv.mkDerivation rec {
        name = "postgrest-${version}";

        version = "9.0.0";

        # https://nixos.wiki/wiki/Packaging/Binaries
        src = pkgs.fetchurl {
          url = "https://github.com/PostgREST/postgrest/releases/download/v${version}/postgrest-v${version}-linux-static-x64.tar.xz";
          sha256 = "sha256-6kgh6heVV7qNcNzcXTiqbVyhfsSV9u5/S3skto6Uzz4=";
        };

        sourceRoot = ".";

        installPhase = ''
        install -m755 -D postgrest $out/bin/postgrest
        '';

        meta = with lib; {
          homepage = "https://postgrest.org";
          description = "REST API for any Postgres database";
          platforms = platforms.linux;
        };
      };
  };
}

_final: prev:
let
  # Nixpkgs commit with mesa 26.0.2
  mesaNixpkgs = fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/4fee11f52b78a777839842faa96c432562ae4a3c.tar.gz";
    sha256 = "16hcmyl92hs57jvfakfpm2xirzw5scp2i89m5xa09r8b4hllqvc1";
  };
  mesaPkgs = import mesaNixpkgs {
    inherit (prev) system;
    config = prev.config;
  };
in
{
  mesa = mesaPkgs.mesa;
}

{ inputs, ... }:
{
  flake.modules.homeManager.desktop-utils =
    { pkgs, ... }:
    let
      src = inputs.yapfast;
      stamp = src.lastModifiedDate;
      yapfast = import "${src}/nix/package.nix" {
        pkgs = pkgs // {
          xorg = {
            inherit (pkgs)
              libxcb
              ;
            libX11 = pkgs.libx11;
            libXcursor = pkgs.libxcursor;
            libXi = pkgs.libxi;
            libXrandr = pkgs.libxrandr;
            libXfixes = pkgs.libxfixes;
            libXdamage = pkgs.libxdamage;
            libXcomposite = pkgs.libxcomposite;
            libXext = pkgs.libxext;
          };
        };
        commit = src.rev;
        date = "${builtins.substring 0 4 stamp}-${builtins.substring 4 2 stamp}-${builtins.substring 6 2 stamp}";
      };
    in
    {
      home.packages = [
        (yapfast.overrideAttrs (old: {
          buildInputs = old.buildInputs ++ [ pkgs.aws-lc ];
        }))
      ];
    };
}

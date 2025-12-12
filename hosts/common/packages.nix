{ pkgs, inputs, ... }:
{
  imports = [ ];
  environment.systemPackages = with pkgs; [
    wget
    git
    deadnix
    usbutils
    nix-init
    nix-update
    nixd
    nil
    nix-inspect
    cachix
    nixfmt-rfc-style
    nvd
    nix-output-monitor
    comma
    nix-tree
    nixos-anywhere
    attic-client
    # Thumbnails
    ffmpeg-headless
    ffmpegthumbnailer
    libheif
    libheif.out
    nufraw
    nufraw-thumbnailer
    gdk-pixbuf
    nixos-update
    inputs.binix.packages.${pkgs.stdenv.hostPlatform.system}.binix-client
  ];

  programs.gdk-pixbuf.modulePackages = [ pkgs.librsvg ];

  environment.pathsToLink = [
    "share/thumbnailers"
  ];

  programs.nh = {
    enable = true;
  };
}

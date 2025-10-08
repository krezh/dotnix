{ pkgs, ... }:
{
  programs.ncspot = {
    enable = true;
    package = pkgs.ncspot.override {
      withCover = true;
      withMPRIS = true;
      withALSA = true;
      withNotify = true;
    };
    settings = {
      bitrate = 320;
      cover_max_scale = 2.0;
      flip_status_indicators = true;
      use_nerdfont = true;
    };
  };
}

{
  lib,
  pkgs,
  config,
  ...
}:
let
  # english with ISO8601 date format
  en_SE = pkgs.fetchurl {
    url = "https://gist.githubusercontent.com/bmaupin/d64368e78cd359d309685fecbe2baf23/raw/e933a0fc2e727aa640f4a1cb1f699622876940fc/en_SE";
    hash = "sha256-ArXL+rMDVWI4Mmcw8xoRlZFXhEeSQL0tLJf5pKEOx3s=";
  };
in
{
  i18n = {
    defaultLocale = lib.mkDefault "en_US.UTF-8";
    supportedLocales = lib.mkDefault [
      "en_US.UTF-8/UTF-8"
      "en_SE.UTF-8/UTF-8"
    ];
    extraLocaleSettings = {
      LC_TIME = "en_US.UTF-8";
    };
    glibcLocales =
      (pkgs.glibcLocales.override {
        allLocales = false;
        locales = config.i18n.supportedLocales;
      }).overrideAttrs
        (_: {
          postUnpack = ''
            cp ${en_SE} $sourceRoot/localedata/locales/en_SE
            echo 'en_SE.UTF-8/UTF-8 \' >> $sourceRoot/localedata/SUPPORTED
          '';
        });
  };
  console.keyMap = "sv-latin1";
  time.timeZone = "Europe/Stockholm";
  environment.variables = {
    TZ = "Europe/Stockholm";
  };
}

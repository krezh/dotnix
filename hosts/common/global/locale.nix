{ lib, ... }: {
  i18n = {
    defaultLocale = lib.mkDefault "en_US.UTF-8";
    supportedLocales = lib.mkDefault [ "en_US.UTF-8/UTF-8" ];

  };
  console.keyMap = "sv-latin1";
  time.timeZone = lib.mkDefault "Europe/Stockholm";
}

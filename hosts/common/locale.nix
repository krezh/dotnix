{
  lib,
  ...
}:
{
  i18n = {
    defaultLocale = lib.mkDefault "en_US.UTF-8";
    supportedLocales = lib.mkDefault [
      "en_US.UTF-8/UTF-8"
    ];
    extraLocaleSettings = {
      LC_TIME = "en_US.UTF-8";
    };
  };
  console.keyMap = "sv-latin1";
  time.timeZone = "Europe/Stockholm";
  environment.variables = {
    TZ = "Europe/Stockholm";
  };
}

{ pkgs, ... }:
{
  services.gpg-agent = {
    enable = false;
    enableBashIntegration = true;
    enableFishIntegration = true;
    enableSshSupport = true;
    enableScDaemon = true;
    pinentry.package = pkgs.pinentry-tty;
    enableExtraSocket = true;
  };

  programs = {
    gpg = {
      enable = true;
      scdaemonSettings = {
        disable-ccid = true;
        reader-port = "Yubico Yubi";
      };
    };
  };
  home = {
    packages = with pkgs; [
      gpg-tui
    ];
  };
}

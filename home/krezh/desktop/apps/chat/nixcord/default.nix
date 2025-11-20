{ inputs, ... }:
{
  imports = [ inputs.nixcord.homeModules.nixcord ];
  programs.nixcord = {
    enable = true; # Enable Nixcord (It also installs Discord)
    vesktop.enable = false; # Vesktop
    discord.enable = true;
    #quickCss = ""; # quickCSS file
    config = {
      useQuickCss = true; # use out quickCSS
      themeLinks = [ ];
      frameless = true; # Set some Vencord options
      plugins = { };
    };
    dorion = {
      enable = false;
      theme = "dark";
      zoom = "1.0";
      blur = "none"; # "none", "blur", or "acrylic"
      sysTray = true;
      openOnStartup = true;
      autoClearCache = true;
      disableHardwareAccel = false;
      rpcServer = true;
      rpcProcessScanner = true;
      pushToTalk = false;
      pushToTalkKeys = [ "RControl" ];
      desktopNotifications = true;
      unreadBadge = true;
    };
    extraConfig = { };
  };
}

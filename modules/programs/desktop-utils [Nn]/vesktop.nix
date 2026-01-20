{
  flake.modules.homeManager.desktop-utils = {pkgs, ...}: {
    programs.vesktop = {
      enable = true;
      package = pkgs.vesktop;
      vencord.useSystem = true;
      settings = {
        appBadge = true;
        arRPC = true;
        enableSplashScreen = false;
        customTitleBar = false;
        disableMinSize = true;
        minimizeToTray = true;
        tray = true;
        staticTitle = false;
        hardwareAcceleration = true;
        hardwareVideoAcceleration = true;
        clickTrayToShowHide = true;
        discordBranch = "stable";
      };
      vencord.settings = {
        autoUpdate = false;
        autoUpdateNotification = false;
        frameless = true;
        notifyAboutUpdates = false;
        transparent = true;
        useQuickCss = true;
        winCtrlQ = false;
        winNativeTitleBar = false;
        eagerPatches = false;
        plugins = {
          GameActivityToggle.enabled = true;
          NoF1.enabled = true;
          "WebRichPresence (arRPC)".enabled = true;
          WebScreenShareFixes.enabled = true;
          ReplaceGoogleSearch = {
            enabled = true;
            customEngineName = "Kagi";
            customEngineURL = "https://kagi.com/search?q=";
          };
        };
      };
    };

    services.arrpc = {
      enable = true;
      package = pkgs.rsrpc;
    };
  };
}

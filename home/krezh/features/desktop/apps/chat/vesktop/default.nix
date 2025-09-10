{ ... }:
{
  hmModules.desktop.vesktop = {
    enable = false;
    settings = {
      appBadge = true;
      arRPC = true;
      enableSplashScreen = false;
      customTitleBar = false;
      disableMinSize = false;
      minimizeToTray = false;
      tray = true;
      splashTheming = true;
      staticTitle = false;
      hardwareAcceleration = true;
      videoHardwareAcceleration = true;
      discordBranch = "stable";
    };
    vencord.settings = {
      autoUpdate = false;
      autoUpdateNotification = true;
      disableMinSize = true;
      enabledThemes = [ ];
      frameless = true;
      notifyAboutUpdates = true;
      plugins = {
        CustomRPC = {
          enabled = false;
        };
        Decor = {
          enabled = true;
          agreedToGuidelines = false;
        };
        FakeNitro = {
          enabled = true;
          disableEmbedPermissionCheck = false;
          emojiSize = 48;
          enableEmojiBypass = true;
          enableStickerBypass = true;
          enableStreamQualityBypass = true;
          hyperLinkText = "{{NAME}}";
          transformCompoundSentence = false;
          transformEmojis = true;
          transformStickers = true;
          useHyperLinks = true;
        };
        Settings = {
          enabled = true;
          settingsLocation = "aboveNitro";
        };
        "WebRichPresence (arRPC)".enabled = true;
        WebScreenShareFixes = {
          enabled = true;
        };
      };
      themeLinks = [
        "https://catppuccin.github.io/discord/dist/catppuccin-mocha-lavender.theme.css"
      ];
      transparent = false;
      useQuickCss = true;
      winCtrlQ = false;
      winNativeTitleBar = false;
    };
  };
}

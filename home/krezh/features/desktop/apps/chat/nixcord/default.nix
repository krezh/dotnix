{
  inputs,
  ...
}:
{
  imports = [
    inputs.nixcord.homeModules.nixcord
  ];
  programs.nixcord = {
    enable = false;
    discord.enable = false;
    vesktop = {
      enable = true;
      settings = {
        discordBranch = "stable";
        staticTitle = false;
        enableSplashScreen = false;
        splashTheming = true;
        arRPC = true;
        minimizeToTray = false;
      };
    };
    config = {
      frameless = true;
      themeLinks = [ "https://catppuccin.github.io/discord/dist/catppuccin-mocha.theme.css" ];
      plugins = {
        fakeNitro.enable = true;
        gameActivityToggle.enable = true;
        noF1.enable = true;
        webRichPresence.enable = true;
      };
    };
  };
}

_: {
  flake.modules.homeManager.browsers =
    { config, ... }:
    {
      programs.zen-browser.profiles.${config.home.username} = {
        pinsForce = true;
        pins = {
          "ProtonMail" = {
            id = "protonmail";
            url = "https://mail.proton.me";
            position = 102;
            isEssential = true;
          };
          "Pushover" = {
            id = "pushover";
            url = "https://client.pushover.net/";
            position = 103;
            isEssential = true;
          };
          "AlertManager" = {
            id = "alertmanager";
            url = "https://alertmanager.plexuz.xyz";
            position = 104;
            isEssential = true;
          };
          "ChatGPT" = {
            id = "chatgpt";
            url = "https://chatgpt.com/";
            position = 106;
            isEssential = false;
          };
          "Claude" = {
            id = "claude";
            url = "https://claude.ai";
            position = 106;
            isEssential = false;
          };
          "Kagi Assistant" = {
            id = "kagiassistant";
            url = "https://kagi.com/assistant";
            position = 107;
            isEssential = false;
          };
        };
      };
    };
}

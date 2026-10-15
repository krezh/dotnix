{
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
          "Gotify" = {
            id = "gotify";
            url = "https://gotify.plexuz.xyz";
            position = 105;
            isEssential = true;
          };
          "ChatGPT" = {
            id = "chatgpt";
            url = "https://chatgpt.com/";
            position = 106;
            isEssential = true;
          };
          "Claude" = {
            id = "claude";
            url = "https://claude.ai";
            position = 106;
            isEssential = true;
          };
          "gemini" = {
            id = "gemini";
            url = "https://gemini.google.com";
            position = 107;
            isEssential = true;
          };
          "Kagi Assistant" = {
            id = "kagiassistant";
            url = "https://kagi.com/assistant";
            position = 108;
            isEssential = true;
          };
          "NixSearch" = {
            id = "nixsearch";
            url = "https://nixsearch.thekoppe.com/";
            position = 109;
            isEssential = true;
          };
        };
      };
    };
}

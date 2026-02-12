let
  user = "krezh";
in
{
  flake.modules.nixos.thor = {
    home-manager.users.${user} = {
      programs.webapps = {
        enable = true;
        apps = {
          "Claude".url = "https://claude.ai";
          "ChatGPT".url = "https://chatgpt.com/";
        };
      };
    };
  };
}

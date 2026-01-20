{inputs, ...}: {
  flake.modules.nixos.thor = {
    home-manager.users.krezh = {
      imports = [inputs.self.modules.homeManager.webapps];
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

{ ... }:

{
  programs.k9s = {
    enable = true;
    aliases = {
      aliases = {
        dp = "deployments";
        sec = "v1/secrets";
        jo = "jobs";
        cr = "clusterroles";
        crb = "clusterrolebindings";
        ro = "roles";
        rb = "rolebindings";
        np = "networkpolicies";
      };
    };
    plugins = import ./plugins.nix;
    settings = {
      k9s = {
        liveViewAutoRefresh = true;
        refreshRate = 2;
        skipLatestRevCheck = true;
        disablePodCounting = false;
        ui = {
          enableMouse = true;
          headless = true;
          logoless = true;
          crumbsless = true;
          reactive = true;
          noIcons = false;
          defaultsToFullScreen = true;
        };
      };
    };
  };
}

{ pkgs, ... }:

{
  modules.shell.k9s = {
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
    package = pkgs.k9s;
    config = {
      k9s = {
        liveViewAutoRefresh = true;
        ui = {
          enableMouse = true;
          headless = true;
          logoless = true;
          crumbsless = true;
          reactive = true;
          noIcons = false;
          defaultsToFullScreen = true;
          skin = "catppuccin-mocha-transparent";
        };
        skipLatestRevCheck = true;
        disablePodCounting = false;
      };
    };
  };
}

{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
with lib;
let
  cfg = config.hmModules.shell.kubernetes;
in
{
  options.hmModules.shell.kubernetes = {
    enable = mkEnableOption "kubernetes";
  };

  config = mkIf cfg.enable {
    home.packages = with pkgs; [
      talosctl
      kubectl
      kubectl-cnpg
      kubectl-node-shell
      kubectl-klock
      kubectl-df-pv
      kustomize
      fluxcd
      stern
      helmfile
      kubernetes-helm
      kubernetes-helmPlugins.helm-diff
      kind
      kubestr
      kubectl-pgo
      cilium-cli
      kubectl-rook-ceph
      k8s-format
      inputs.kauth.packages.${pkgs.stdenv.hostPlatform.system}.kauth
    ];

    programs.fish = {
      shellAbbrs = {
        k = "kubectl";
      };
      shellAliases = {
        kubectl = "kubecolor";
      };
    };
    programs.k9s = {
      enable = true;
      aliases = {
        aliases = {
          dp = "deployments";
          sec = "v1/secrets";
          cm = "configmaps";
          ns = "namespaces";
          sv = "serviceaccounts";
          jo = "jobs";
          cr = "clusterroles";
          crb = "clusterrolebindings";
          ro = "roles";
          rb = "rolebindings";
          np = "networkpolicies";
          ing = "ingresses";
          po = "pods";
          svc = "services";
          no = "nodes";
          ds = "daemonsets";
          sts = "statefulsets";
          cj = "cronjobs";
          ep = "endpoints";
          htr = "httproutes";
          gw = "gateway";
          psp = "podsecuritypolicies";
          sp = "securitypolicies";
        };
      };
      plugins = { };
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

    hmModules.shell.krew.enable = true;
    hmModules.shell.kubectx.enable = true;
    hmModules.shell.talswitcher.enable = true;
    catppuccin.k9s.enable = true;
    catppuccin.k9s.transparent = true;

    programs = {
      kubecolor = {
        enable = true;
        package = pkgs.kubecolor;
      };
      kubeswitch = {
        enable = true;
        enableFishIntegration = true;
        package = pkgs.kubeswitch;
      };
    };
  };
}

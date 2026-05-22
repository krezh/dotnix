{ inputs, ... }:
{
  flake.modules.homeManager.kubernetes =
    { pkgs, lib, ... }:
    {
      home.file.".kube/kuberc".text = lib.generators.toYAML { } {
        apiVersion = "kubectl.config.k8s.io/v1beta1";
        kind = "Preference";
        defaults = [
          {
            command = "apply";
            options = [
              {
                name = "server-side";
                default = "true";
              }
            ];
          }
        ];
      };

      home.packages = with pkgs; [
        talosctl
        talswitcher
        kubectl
        kubectl-node-shell
        kubectl-klock
        kubectl-df-pv
        kubectl-pgo
        kubectl-rook-ceph
        kustomize
        fluxcd
        stern
        helmfile
        helm-ls
        kubernetes-helm
        kubernetes-helmPlugins.helm-diff
        kind
        kubestr
        cilium-cli
        k8s-format
        kubectx
        klim
        egctl
        inputs.kauth.packages.${pkgs.stdenv.hostPlatform.system}.kauth
        lfk
      ];

      catppuccin = {
        k9s.enable = true;
        k9s.transparent = true;
      };

      programs.fish = {
        shellAbbrs.k = "kubectl";
        shellAliases.kubectl = "kubecolor";
      };

      programs = {
        k9s = {
          enable = true;
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
          views = {
            "v1/pods" = {
              "sortColumn" = "NAMESPACE:asc";
            };
          };
          plugins = { };
          aliases.aliases = {
            dp = "deployments";
            sec = "v1/secrets";
            cm = "configmaps";
            ns = "namespaces";
            jo = "jobs";
            cr = "clusterroles";
            crb = "clusterrolebindings";
            ro = "roles";
            rb = "rolebindings";
            np = "networkpolicies";
          };
        };
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

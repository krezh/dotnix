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
        kubernetes-helm
        kubernetes-helmPlugins.helm-diff
        kind
        kubestr
        cilium-cli
        k8s-format
        kubectx
        klim
        inputs.kauth.packages.${pkgs.stdenv.hostPlatform.system}.kauth
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
          settings.k9s = {
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
          plugins = {
            toggle-helmrelease = {
              shortCut = "Shift-T";
              confirm = true;
              scopes = [ "helmreleases" ];
              description = "Toggle to suspend or resume a HelmRelease";
              command = "bash";
              background = true;
              args = [
                "-c"
                "suspended=$(kubectl --context $CONTEXT get helmreleases -n $NAMESPACE $NAME -o=custom-columns=TYPE:.spec.suspend | tail -1); verb=$([ $suspended = \"true\" ] && echo \"resume\" || echo \"suspend\"); flux $verb helmrelease --context $CONTEXT -n $NAMESPACE $NAME"
              ];
            };
            toggle-kustomization = {
              shortCut = "Shift-T";
              confirm = true;
              scopes = [ "kustomizations" ];
              description = "Toggle to suspend or resume a Kustomization";
              command = "bash";
              background = true;
              args = [
                "-c"
                "suspended=$(kubectl --context $CONTEXT get kustomizations -n $NAMESPACE $NAME -o=custom-columns=TYPE:.spec.suspend | tail -1); verb=$([ $suspended = \"true\" ] && echo \"resume\" || echo \"suspend\"); flux $verb kustomization --context $CONTEXT -n $NAMESPACE $NAME"
              ];
            };
            reconcile-git = {
              shortCut = "Shift-R";
              confirm = false;
              description = "Flux reconcile";
              scopes = [ "gitrepositories" ];
              command = "bash";
              background = true;
              args = [
                "-c"
                "flux reconcile source git --context $CONTEXT -n $NAMESPACE $NAME"
              ];
            };
            reconcile-hr = {
              shortCut = "Shift-R";
              confirm = false;
              description = "Flux reconcile";
              scopes = [ "helmreleases" ];
              command = "bash";
              background = true;
              args = [
                "-c"
                "flux reconcile helmrelease --context $CONTEXT -n $NAMESPACE $NAME"
              ];
            };
            reconcile-ks = {
              shortCut = "Shift-R";
              confirm = false;
              description = "Flux reconcile";
              scopes = [ "kustomizations" ];
              command = "bash";
              background = true;
              args = [
                "-c"
                "flux reconcile kustomization --context $CONTEXT -n $NAMESPACE $NAME"
              ];
            };
          };
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

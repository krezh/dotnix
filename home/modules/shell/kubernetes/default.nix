{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:
let
  cfg = config.hmModules.shell.kubernetes;
  system = pkgs.stdenv.hostPlatform.system;
in
{
  options.hmModules.shell.kubernetes = {
    enable = lib.mkEnableOption "kubernetes";
  };

  config = lib.mkIf cfg.enable {

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
      kubevirt
      inputs.kauth.packages.${system}.kauth
    ];

    catppuccin = {
      k9s.enable = true;
      k9s.transparent = true;
    };

    programs.fish = {
      shellAbbrs = {
        k = "kubectl";
      };
      shellAliases = {
        kubectl = "kubecolor";
      };
    };

    hmModules.shell.kubectx.enable = true;
    hmModules.shell.talswitcher.enable = true;

    programs = {
      k9s = {
        enable = true;
        settings = import ./k9s/settings.nix;
        plugins = import ./k9s/plugins.nix;
        aliases = import ./k9s/aliases.nix;
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

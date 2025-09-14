{ pkgs, ... }:

{
  # bin packages
  talswitcher = pkgs.callPackage ./bin/talswitcher { };
  gowall = pkgs.callPackage ./bin/gowall { };
  yaml2nix = pkgs.callPackage ./bin/yaml2nix { };
  kubectl-df-pv = pkgs.callPackage ./bin/kubectl-df-pv { };
  gh-poi = pkgs.callPackage ./bin/gh-poi { };
  recshot = pkgs.callPackage ./bin/recshot { };
  talosctl = pkgs.callPackage ./bin/talosctl { };
  kubectl-pgo = pkgs.callPackage ./bin/kubectl-pgo { };
  kubectl-volsync = pkgs.callPackage ./bin/kubectl-volsync { };
  kubectl-rook-ceph = pkgs.callPackage ./bin/kubectl-rook-ceph { };
  kubectl-browse-pvc = pkgs.callPackage ./bin/kubectl-browse-pvc { };
  hypr-showkey = pkgs.callPackage ./bin/hypr-showkey { };
  fluxcd = pkgs.callPackage ./bin/fluxcd { };
  kubestr = pkgs.callPackage ./bin/kubestr { };
  hyprmon = pkgs.callPackage ./bin/hyprmon { };
  wowup = pkgs.callPackage ./bin/wowup { };

  # script packages
  volume_script_hyprpanel = pkgs.callPackage ./scripts/volume_script_hyprpanel { };
  tlk = pkgs.callPackage ./scripts/tlk { };
  volume_script = pkgs.callPackage ./scripts/volume_script { };
  brightness_script_hyprpanel = pkgs.callPackage ./scripts/brightness_script_hyprpanel { };
  brightness_script = pkgs.callPackage ./scripts/brightness_script { };
}

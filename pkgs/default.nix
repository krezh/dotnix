{ pkgs, ... }:
let
  inherit (pkgs) callPackage;
in
{
  # bin packages
  talswitcher = callPackage ./bin/talswitcher { };
  gowall = callPackage ./bin/gowall { };
  kubectl-df-pv = callPackage ./bin/kubectl-df-pv { };
  gh-poi = callPackage ./bin/gh-poi { };
  recshot = callPackage ./bin/recshot { };
  talosctl = callPackage ./bin/talosctl { };
  kubectl-pgo = callPackage ./bin/kubectl-pgo { };
  kubectl-rook-ceph = callPackage ./bin/kubectl-rook-ceph { };
  kubectl-browse-pvc = callPackage ./bin/kubectl-browse-pvc { };
  hypr-showkey = callPackage ./bin/hypr-showkey { };
  fluxcd = callPackage ./bin/fluxcd { };
  kubestr = callPackage ./bin/kubestr { };
  hyprmon = callPackage ./bin/hyprmon { };
  hypr-slurp = callPackage ./bin/hypr-slurp { };

  # script packages
  volume_script_hyprpanel = callPackage ./scripts/volume_script_hyprpanel { };
  tlk = callPackage ./scripts/tlk { };
  volume_script = callPackage ./scripts/volume_script { };
  brightness_script_hyprpanel = callPackage ./scripts/brightness_script_hyprpanel { };
  brightness_script = callPackage ./scripts/brightness_script { };
}

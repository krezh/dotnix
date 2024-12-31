#!/usr/bin/env bash

show_help() {
  echo "Usage: $0 [-u|--user <username>]"
  echo "If no argument is provided, it logs in with your github account."
  exit 0
}

main() {
  local USER=""
  local CONFIG_DIR="$HOME/.config/tlk"
  local TELEPORT_PROXY

  if [ -f "$CONFIG_DIR/proxy" ]; then
    TELEPORT_PROXY="$(<"$CONFIG_DIR/proxy")"
  else
    echo "Proxy file not found: $CONFIG_DIR/proxy"
    exit 1
  fi

  while [[ "$#" -gt 0 ]]; do
    case $1 in
      -u|--user) USER=$2; shift ;;
      -h|--help) show_help ;;
      *) echo "Unknown parameter passed: $1"; show_help ;;
    esac
    shift
  done

  if [ -n "$USER" ]; then
    tsh login --proxy="$TELEPORT_PROXY" --auth=local --user="$USER"
    local CLUSTER
    local CLUSTERS
    CLUSTERS=$(tsh kube ls -q -f json | jq -r '.[].kube_cluster_name' | tr '\n' ' ')
    # shellcheck disable=SC2086
    CLUSTER=$(gum choose --header="Select Kubernetes Cluster:" $CLUSTERS)
    tsh kube login "$CLUSTER"
  else
    tsh login --proxy="$TELEPORT_PROXY" --auth=github
    local CLUSTER
    local CLUSTERS
    CLUSTERS=$(tsh kube ls -q -f json | jq -r '.[].kube_cluster_name' | tr '\n' ' ')
    # shellcheck disable=SC2086
    CLUSTER=$(gum choose --header="Select Kubernetes Cluster:" $CLUSTERS)
    tsh kube login "$CLUSTER"
  fi
}

main "$@"
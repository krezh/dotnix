#!/usr/bin/env bash

# Variables
CONFIG_DIR="$HOME/.config/tlk"
CONFIG_FILE="$CONFIG_DIR/config.json"
DEPENDENCIES=("yq" "gum" "tsh" "jq")
DEFAULT_PROXY_TTL="1d"
DEFAULT_PROXY_URL="example.com"

show_help() {
  echo "Usage: $0 [-c|--create-config, -h|--help, -l|--logout, -s|--set <parameter>, -u|--user <username>]"
  echo "Options:"
  echo "  -c, --create-config      Create a configuration file for Teleport proxy."
  echo "  -h, --help               Show this help message."
  echo "  -l, --logout             Log out from the Teleport cluster."
  echo "  -s, --set <parameter>    Set a configuration parameter (proxy_url or proxy_ttl)."
  echo "  -u, --user <username>    Specify the username for login."
  echo "Description:"
  echo "  This script logs into a Teleport cluster using the tsh CLI."
  echo "  If the -c option is provided, it creates a configuration file for the Teleport proxy."
  echo "  If the -h option is provided, it shows this help message."
  echo "  If the -l option is provided, it logs out from the Teleport cluster."
  echo "  If the -s option is provided, it allows setting configuration parameters."
  echo "  If the -u option is provided, it logs in with the specified username."
  echo "  If no options are provided, it logs in with your GitHub account."
  exit 0
}

check_dependencies() {
  for cmd in "${DEPENDENCIES[@]}"; do
    if ! command -v "$cmd" &> /dev/null; then
      echo "$cmd is not installed. Please install it to use this script."
      exit 1
    fi
  done
}

timeConvert() {
  local input="$1"
  local days=0 hours=0 mins=0 total=0
  if [[ -z "$input" ]]; then
    echo "Input cannot be empty."
    exit 1
  fi
  # Extract days, hours, and minutes if present (only first match for each)
  if [[ $input =~ ([0-9]+)d ]]; then
    days="${BASH_REMATCH[1]}"
  fi
  if [[ $input =~ ([0-9]+)h ]]; then
    hours="${BASH_REMATCH[1]}"
  fi
  if [[ $input =~ ([0-9]+)m ]]; then
    mins="${BASH_REMATCH[1]}"
  fi

  total=$((days * 1440 + hours * 60 + mins))
  echo "$total"
}

validateURL() {
  local url="$1"
  if [[ "$url" =~ ^https?:// ]]; then
    echo "Invalid URL: $url"
    echo "Proxy URL should not start with http:// or https://"
    exit 1
  fi
}

createConfig() {
  if [ ! -f "$CONFIG_FILE" ]; then
    mkdir -p "$CONFIG_DIR"
    touch "$CONFIG_FILE"
    yq -iPo json ".proxy.url=\"$DEFAULT_PROXY_URL\"" "$CONFIG_FILE"
    yq -iPo json ".proxy.ttl=$(timeConvert $DEFAULT_PROXY_TTL)" "$CONFIG_FILE"
    echo "Configuration created at $CONFIG_FILE"
  else
    echo "Configuration already exists at $CONFIG_FILE"
  fi
}

setConfig() {
  if [ ! -f "$CONFIG_FILE" ]; then
    echo "Configuration file not found. Please create it using the -c option."
    exit 1
  fi

  while [[ "$#" -gt 0 ]]; do
    case $1 in
      -s|--set) 
        if [[ "$2" == "proxy_url" ]]; then
          PROXY_URL=$(gum input --placeholder="Enter Proxy URL" --value="$PROXY_URL")
          validateURL "$PROXY_URL"
          if [[ -z "$PROXY_URL" ]]; then
            echo "Proxy URL cannot be empty."
            exit 1
          fi
          yq -iPo json ".proxy.url=\"$PROXY_URL\"" "$CONFIG_FILE"
        elif [[ "$2" == "proxy_ttl" ]]; then
          PROXY_TTL=$(gum input --placeholder="Enter Proxy TTL (1d24h60m format)" --value="$PROXY_TTL")
          yq -iPo json ".proxy.ttl=\"$PROXY_TTL\"" "$CONFIG_FILE"
        else
          echo "Unknown config parameter: $2"
          exit 1
        fi
        shift ;;
      *)
    esac
    shift
  done

  echo "Configuration updated in $CONFIG_FILE"
}

fixConfig() {
  if [ ! -f "$CONFIG_FILE" ]; then
    echo "Configuration file not found. Please create it using the -c option."
    exit 1
  fi

  PROXY_URL=$(jq -r '.proxy.url' "$CONFIG_FILE")
  if [ "$PROXY_URL" == "null" ]; then
    echo "Proxy URL not set in the configuration file. Please update $CONFIG_FILE or use -s proxy_url."
    exit 1
  fi

  PROXY_TTL=$(jq -r '.proxy.ttl' "$CONFIG_FILE")
  if [ "$PROXY_TTL" == "null" ]; then
    echo "Proxy TTL not set in the configuration file. Defaulting to $DEFAULT_PROXY_TTL."
    PROXY_TTL=$(timeConvert $DEFAULT_PROXY_TTL)
  fi
}

logout() {
  tsh logout
}

main() {
  local user=""

  while [[ "$#" -gt 0 ]]; do
    case $1 in
      -c|--create-config) createConfig; exit 0 ;;
      -h|--help) show_help ;;
      -l|--logout) logout; exit 0 ;;
      -s|--set) setConfig "$@"; exit 0 ;;
      -u|--user) user=$2; shift ;;
      *) echo "Unknown parameter passed: $*"; show_help ;;
    esac
    shift
  done

  if [ -n "$user" ]; then
    tsh login --proxy="$PROXY_URL" --auth=local --user="$user" --ttl="$(timeConvert "$PROXY_TTL")"
    local cluster
    local clusters
    clusters=$(tsh kube ls -q -f json | jq -r '.[].kube_cluster_name' | tr '\n' ' ')
    # shellcheck disable=SC2086
    cluster=$(gum choose --header="Select Kubernetes Cluster:" $clusters)
    tsh kube login "$cluster"
  else
    tsh login --proxy="$PROXY_URL" --auth=github --ttl="$(timeConvert "$PROXY_TTL")"
    local cluster
    local clusters
    clusters=$(tsh kube ls -q -f json | jq -r '.[].kube_cluster_name' | tr '\n' ' ')
    # shellcheck disable=SC2086
    cluster=$(gum choose --header="Select Kubernetes Cluster:" $clusters)
    tsh kube login "$cluster"
  fi
}

check_dependencies
fixConfig
main "$@"
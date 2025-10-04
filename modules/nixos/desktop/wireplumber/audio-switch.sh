#!/usr/bin/env bash

# Audio switching script for toggling between audio devices
# Uses wpctl (WirePlumber control) to switch between audio devices

set -euo pipefail

# Configuration - these will be replaced by the module
HEADSET_NAME="@PRIMARY_DEVICE_NAME@"
SPEAKERS_NAME="@SECONDARY_DEVICE_NAME@"

# Function to get node ID by display name
get_node_id() {
    local display_name="$1"
    wpctl status | grep "$display_name" | grep -o "[0-9]\+" | head -1
}

# Function to get current default sink ID and name
get_default_sink() {
    local line
    line=$(wpctl status | grep -A 20 "Audio" | grep "\*" | grep -v "Microphone\|Source" | head -1)
    if [[ -n "$line" ]]; then
        local id name
        id=$(echo "$line" | grep -o "[0-9]\+" | head -1)
        name=$(echo "$line" | sed 's/.*[0-9]\+\.\s*\([^[]*\)\s*\[.*/\1/' | sed 's/[[:space:]]*$//')
        echo "$id:$name"
    fi
}

# Function to check if a sink is available by name
is_sink_available() {
    local sink_name="$1"
    local node_id
    node_id=$(get_node_id "$sink_name")
    [[ -n "$node_id" ]]
}

# Function to switch to a specific sink by name
switch_to_sink() {
    local sink_name="$1"
    local node_id
    node_id=$(get_node_id "$sink_name")

    if [[ -n "$node_id" ]]; then
        echo "Switching to $sink_name (ID: $node_id)..."
        wpctl set-default "$node_id"

        # Send notification if available
        if command -v notify-send &> /dev/null; then
            notify-send "Audio Switched" "Now using: $sink_name" -t 2000
        fi
    else
        echo "Error: $sink_name is not available"
        if command -v notify-send &> /dev/null; then
            notify-send "Audio Switch Failed" "$sink_name is not available" -t 3000
        fi
        return 1
    fi
}

# Function to toggle between devices
toggle() {
    local current_info
    current_info=$(get_default_sink)
    local current_name="${current_info#*:}"

    if [[ "$current_name" == *"@PRIMARY_DEVICE_NAME@"* ]]; then
        # Currently using primary device, switch to secondary device
        switch_to_sink "$SPEAKERS_NAME"
    else
        # Currently using secondary or unknown, try primary device first
        if is_sink_available "$HEADSET_NAME"; then
            switch_to_sink "$HEADSET_NAME"
        else
            echo "@PRIMARY_DEVICE_NAME@ not available, staying on current device"
        fi
    fi
}

# Function to show help
show_help() {
    cat << EOF
Audio Switch Script - Toggle between audio output devices

Usage: $0 [COMMAND]

Commands:
    toggle, t       Toggle between @PRIMARY_DEVICE_NAME@ and @SECONDARY_DEVICE_NAME@
    help, -h        Show this help message

Default action (no arguments): toggle

Examples:
    $0              # Toggle between devices
    $0 toggle       # Toggle between devices
    $0 t            # Toggle between devices (short form)
EOF
}

# Main command handling
case "${1:-toggle}" in
    "toggle"|"t"|"")
        toggle
        ;;
    "help"|"--help"|"-h")
        show_help
        ;;
    *)
        echo "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac

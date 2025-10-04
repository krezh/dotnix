#!/usr/bin/env bash

# Audio switching script for Argon Speakers and A50 Game Audio
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

        echo "Successfully switched to $sink_name"

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

# Function to auto-switch based on availability
auto_switch() {
    if is_sink_available "$HEADSET_NAME"; then
        switch_to_sink "$HEADSET_NAME"
    elif is_sink_available "$SPEAKERS_NAME"; then
        switch_to_sink "$SPEAKERS_NAME"
    else
        echo "Error: No audio devices available"
        if command -v notify-send &> /dev/null; then
            notify-send "Audio Error" "No audio devices available" -t 3000
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
            echo "@PRIMARY_DEVICE_NAME@ not available, staying on @SECONDARY_DEVICE_NAME@"
        fi
    fi
}

# Function to show current status
status() {
    echo "=== Audio Device Status ==="
    echo

    local current_info
    current_info=$(get_default_sink)
    if [[ -n "$current_info" ]]; then
        local current_id="${current_info%:*}"
        local current_name="${current_info#*:}"
        echo "Current default sink: $current_name (ID: $current_id)"
    else
        echo "Current default sink: unknown"
    fi
    echo

    echo "Device availability:"
    local headset_id speakers_id
    headset_id=$(get_node_id "$HEADSET_NAME")
    speakers_id=$(get_node_id "$SPEAKERS_NAME")

    if [[ -n "$headset_id" ]]; then
        echo "  ✓ $HEADSET_NAME (ID: $headset_id)"
    else
        echo "  ✗ $HEADSET_NAME (unavailable)"
    fi

    if [[ -n "$speakers_id" ]]; then
        echo "  ✓ $SPEAKERS_NAME (ID: $speakers_id)"
    else
        echo "  ✗ $SPEAKERS_NAME (unavailable)"
    fi
    echo

    echo "Full audio status:"
    wpctl status
}

# Function to show help
show_help() {
    cat << EOF
Audio Switch Script - Control audio output devices

Usage: $0 [COMMAND]

Commands:
    headset         Switch to @PRIMARY_DEVICE_NAME@
    speakers        Switch to @SECONDARY_DEVICE_NAME@
    toggle          Toggle between @PRIMARY_DEVICE_NAME@ and @SECONDARY_DEVICE_NAME@
    auto            Auto-switch to best available device
    status          Show current audio status
    help            Show this help message

Examples:
    $0 headset      # Switch to @PRIMARY_DEVICE_NAME@
    $0 speakers     # Switch to @SECONDARY_DEVICE_NAME@
    $0 toggle       # Toggle between devices
    $0 auto         # Auto-select best device
    $0 status       # Show current status
EOF
}

# Main command handling
case "${1:-auto}" in
    "headset"|"h")
        switch_to_sink "$HEADSET_NAME"
        ;;
    "speakers"|"s")
        switch_to_sink "$SPEAKERS_NAME"
        ;;
    "toggle"|"t")
        toggle
        ;;
    "auto"|"a")
        auto_switch
        ;;
    "status")
        status
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

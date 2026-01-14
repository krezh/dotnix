{
  flake.modules.nixos.thor =
    { pkgs, ... }:
    {
      environment.systemPackages = [
        (pkgs.writeShellScriptBin "usb-info" ''
          #!/usr/bin/env bash

          # Colors
          RED='\033[0;31m'
          GREEN='\033[0;32m'
          BLUE='\033[0;34m'
          NC='\033[0m'

          echo -e "''${BLUE}=== USB Controllers ===''${NC}"
          for ctrl in /sys/bus/pci/drivers/xhci_hcd/0000:*/usb*; do
            if [ -d "$ctrl" ]; then
              bus=$(basename "$ctrl" | sed 's/usb//')
              pci=$(basename $(dirname "$ctrl"))
              vendor=$(${pkgs.pciutils}/bin/lspci -s $pci | cut -d: -f3-)

              if echo "$vendor" | grep -qi "asmedia"; then
                echo -e "Bus $bus → ''${RED}$vendor [AVOID]''${NC}"
              else
                echo -e "Bus $bus → ''${GREEN}$vendor''${NC}"
              fi
            fi
          done

          echo ""
          echo -e "''${BLUE}=== Connected Devices ===''${NC}"
          printf "%-4s %-6s %-50s %-15s\n" "Bus" "Dev" "Product" "Driver"
          printf "%-4s %-6s %-50s %-15s\n" "---" "---" "-------" "------"

          for device in /sys/bus/usb/devices/*/; do
            if [ -f "$device/product" ]; then
              product=$(cat "$device/product" 2>/dev/null | tr -d '\n' | cut -c1-50)
              driver=$(basename $(readlink "$device/driver" 2>/dev/null) 2>/dev/null)
              busnum=$(cat "$device/busnum" 2>/dev/null)
              devnum=$(cat "$device/devnum" 2>/dev/null)

              # Skip root hubs and generic hubs
              if [ "$devnum" != "001" ] && [ -n "$product" ] && ! echo "$product" | grep -qi "hub\|host controller"; then
                if [ "$busnum" = "3" ] || [ "$busnum" = "4" ]; then
                  printf "''${RED}%-4s %-6s %-50s %-15s''${NC}\n" "$busnum" "$devnum" "$product" "''${driver:-none}"
                else
                  printf "%-4s %-6s %-50s %-15s\n" "$busnum" "$devnum" "$product" "''${driver:-none}"
                fi
              fi
            fi
          done | sort -n
        '')
      ];
    };
}

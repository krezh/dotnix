# Steam Deck OLED NixOS Installation Guide

## Prerequisites

**Hardware Required:**

- External monitor (REQUIRED - [Jovian-NixOS #39](https://github.com/Jovian-Experiments/Jovian-NixOS/issues/39))
- USB-C hub with HDMI/DisplayPort
- External keyboard and mouse
- 8GB+ USB drive
- Another computer to prepare installer

**Before Starting:**

- Backup game saves (verify Steam Cloud sync)
- Note WiFi passwords

## Step 1: Extract OLED Mura Correction Images

**CRITICAL:** These display calibration images may not be replaceable.

On current SteamOS:

```bash
sudo steamos-readonly disable
sudo pacman -S galileo-mura
sudo steamos-readonly enable

galileo-mura-extractor
# Creates /tmp/mura/blob.tar
cp /tmp/mura/blob.tar ~/Desktop/
```

Copy the file to your nix-config:

```bash
# home/modules/steamdeck-mura/24F70865C8.tar
# (Replace with your actual serial number from galileo-mura-extractor)
```

## Step 2: Update BIOS

In SteamOS:

- Settings → System → Check for updates
- Install any BIOS/firmware updates

## Step 3: Prepare Installation Media

On another computer:

**Option A: Standard NixOS ISO (recommended)**

```bash
# Download GNOME ISO from https://nixos.org/download/
# Flash to USB:
sudo dd if=nixos-*.iso of=/dev/sdX bs=4M status=progress oflag=sync
```

**Option B: Build Jovian ISO**

```bash
git clone https://github.com/Jovian-Experiments/Jovian-NixOS
cd Jovian-NixOS
nix-build -A isoGnome
# ISO in result/iso/
```

## Step 4: Boot from USB

1. Connect USB-C hub to Steam Deck
2. Connect external monitor, keyboard, mouse to hub
3. Insert USB drive into hub
4. **Access boot menu:**
   - Power off Steam Deck
   - Hold **Volume Down** button
   - Tap **Power** button
   - Release when boot screen appears
5. Select USB drive

## Step 5: Partition with Disko

In the installer:

```bash
# Connect to WiFi
nmcli device wifi list
nmcli device wifi connect "SSID" password "PASSWORD"

# Enter devshell
nix develop github:krezh/nix-config

# WARNING: This ERASES /dev/nvme0n1
# Partition layout (500GB SSD):
# - 1GB boot
# - 80GB root
# - ~419GB home
# - 16GB swap file (created automatically)
partition steamdeck

# Verify mounts
mount | grep /mnt
```

## Step 6: Install NixOS

```bash
# Install (still in devshell)
install steamdeck

# Set root password when prompted
```

## Step 7: First Boot

```bash
sudo reboot
# Remove USB drive when system shuts down
```

System boots directly to **Gaming Mode** (Steam Deck UI).

Complete Steam's first-time setup and connect to WiFi.

## Step 8: Switch to Desktop Mode

From Gaming Mode:

1. Press **Steam button**
2. Navigate to **Power**
3. Select **Switch to Desktop**
4. Hyprland launches

## Post-Installation

SSH or in desktop mode:

```bash
# Update system
sudo nixos-rebuild switch --flake /etc/nixos/nix-config#steamdeck

# Check firmware
fwupdmgr get-devices
fwupdmgr refresh
fwupdmgr get-updates
```

## Configuration Details

Jovian automatically handles:

- Kernel: 6.16.12-valve4 (OLED audio support)
- Audio: PipeWire + Steam Deck DSP
- Graphics: AMD drivers, 32-bit support
- Steam: Gaming Mode, gamescope
- Hardware: Controllers, fan control, TDP/GPU tuning
- System: Bluetooth, kernel params, SD card automount

Your configuration provides:

- Boot setup and swap file
- NetworkManager (required for Steam UI)
- Hyprland (desktop environment)
- GameMode and Proton-GE (optional enhancements)

## Troubleshooting

**WiFi not working:**

```bash
dmesg | grep ath11k  # Check for firmware errors
```

**Audio not working:**

```bash
uname -r  # Verify kernel 6.16+
ls /nix/store/*/lib/firmware/amd/sof/  # Check firmware files
```

**Bluetooth suspend bug:** Don't turn Bluetooth off or system won't suspend until reboot. Keep Bluetooth enabled.

## References

- [Jovian-NixOS Documentation](https://jovian-experiments.github.io/Jovian-NixOS/)
- [Steam Deck OLED Support (Issue #227)](https://github.com/Jovian-Experiments/Jovian-NixOS/issues/227)
- [Disko Documentation](https://github.com/nix-community/disko)

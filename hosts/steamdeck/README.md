# Steam Deck OLED NixOS Installation

## Hardware

- External monitor
- USB-C hub with HDMI/DisplayPort
- External keyboard and mouse
- 8GB+ USB drive

## Step 1: Extract Mura Correction Images

On current SteamOS:

```bash
sudo steamos-readonly disable
sudo pacman -S galileo-mura
sudo steamos-readonly enable
galileo-mura-extractor
cp /tmp/mura/blob.tar ~/Desktop/
```

Copy to `home/modules/steamdeck-mura/<SERIAL>.tar`

## Step 2: Update BIOS

Settings → System → Check for updates

## Step 3: Prepare Installation Media

Download latest ISO from [releases](https://github.com/krezh/nix-config/releases)

```bash
sudo dd if=livecd-*.iso of=/dev/sdX bs=4M status=progress oflag=sync
```

## Step 4: Boot from USB

1. Connect USB-C hub, monitor, keyboard, mouse
2. Insert USB drive
3. Power off
4. Hold Volume Down + tap Power
5. Select USB drive

## Step 5: Installation

### Option A: nixos-anywhere (Remote)

On Steam Deck:

```bash
nmcli device wifi connect "SSID" password "PASSWORD"
ip -brief addr show
```

On your computer:

```bash
nix run github:nix-community/nixos-anywhere -- \
  --flake github:krezh/nix-config#steamdeck \
  --target-host nixos@<IP>
```

### Option B: disko-install (Local)

On Steam Deck:

```bash
nmcli device wifi connect "SSID" password "PASSWORD"
nix develop github:krezh/nix-config
disko-install steamdeck
```

## Step 6: First Boot

```bash
sudo reboot
```

Complete Steam setup in Gaming Mode.

## Step 7: Switch to Desktop

Steam button → Power → Switch to Desktop

## Post-Installation

```bash
sudo nixos-rebuild switch --flake github:krezh/nix-config#steamdeck
fwupdmgr refresh && fwupdmgr get-updates
```

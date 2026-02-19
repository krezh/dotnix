{ inputs, ... }:
{
  flake.modules.nixos.jotunheim =
    { pkgs, ... }:
    {
      imports = with inputs.self.modules.nixos; [
        system-base
        openssh
        krezh
      ];

      home-manager.users.root = {
        imports = with inputs.self.modules.homeManager; [
          system-base
        ];
      };

      networking.hostId = "1321fdc7";

      boot = {
        zfs.extraPools = [ "tank" ];
        # Boot configuration - GRUB with ZFS support
        loader.grub = {
          enable = true;
          efiSupport = true;
          efiInstallAsRemovable = true;
          zfsSupport = true;
          device = "nodev";
          mirroredBoots = [
            {
              devices = [ "nodev" ];
              path = "/boot/efi";
            }
            {
              devices = [ "nodev" ];
              path = "/boot/efi-fallback";
            }
          ];
        };
        supportedFilesystems = [ "zfs" ];
      };

      # Ensure /mnt/tank exists for the data pool
      systemd.tmpfiles.rules = [
        "d /mnt/tank 0755 root root -"
      ];

      services = {
        zfs = {
          autoScrub = {
            enable = true;
            pools = [
              "rpool" # OS pool
              "tank" # Data pool
            ];
          };
          trim.enable = true;
        };

        # SSH configuration
        openssh.settings = {
          PermitRootLogin = "prohibit-password";
          PasswordAuthentication = false;
        };

        # NFS server configuration
        nfs.server = {
          enable = true;
          exports = ''
            /mnt/tank/ipxe *(sec=sys,rw,anonuid=0,anongid=0,all_squash,insecure,no_subtree_check)
            /mnt/tank/media 10.10.0.0/27(sec=sys,rw,anonuid=0,anongid=0,all_squash,insecure,no_subtree_check)
            /mnt/tank/k8s 10.10.0.0/27(sec=sys,rw,anonuid=0,anongid=0,all_squash,insecure,no_subtree_check)
            /mnt/tank/volsync 10.10.0.0/27(sec=sys,rw,anonuid=0,anongid=0,all_squash,insecure,no_subtree_check)
            /mnt/tank/crunchy-postgres 10.10.0.0/27(sec=sys,rw,insecure,no_subtree_check)
            /mnt/tank/kopia *(sec=sys,rw,anonuid=0,anongid=0,all_squash,insecure,no_subtree_check)
          '';
        };

        # Samba configuration
        samba = {
          enable = true;
          openFirewall = true;
          settings = {
            global = {
              "workgroup" = "WORKGROUP";
              "server string" = "FreeNAS Server";
              "netbios name" = "JOTUNHEIM";
              "security" = "user";
              "guest account" = "nobody";
              "create mask" = "0664";
              "directory mask" = "0775";
              "local master" = "yes";
              "log level" = "1";
              "server smb encrypt" = "default";
            };
            shares = {
              "path" = "/mnt/tank/shares/%U";
              "browseable" = "yes";
              "read only" = "no";
              "comment" = "";
            };
          };
        };
      };

      users.users.root.openssh.authorizedKeys.keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIANNodE0rg2XalK+tfsqfPwLdBRJIx15IjGwkr5Bud+W"
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEMe4X4oNA8PRUHrOk5RIrpxpzzcBvJyQa8PyaQj3BPp"
      ];

      # Firewall configuration
      networking.firewall = {
        enable = true;
        allowedTCPPorts = [
          22 # SSH
          2049 # NFS
          445 # SMB
          139 # SMB
        ];
        allowedUDPPorts = [
          137 # NetBIOS Name Service
          138 # NetBIOS Datagram Service
        ];
      };

      # System packages
      environment.systemPackages = with pkgs; [
        neovim
        git
        htop
        iotop
        smartmontools
        zfs
      ];

      # Locale and timezone
      time.timeZone = "Europe/Stockholm";
      console.keyMap = "sv-latin1";
      i18n.defaultLocale = "en_US.UTF-8";

      system.stateVersion = "24.05";
    };
}

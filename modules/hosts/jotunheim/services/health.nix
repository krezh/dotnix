{
  flake.modules.nixos.jotunheim =
    { pkgs, ... }:
    {
      services = {
        smartd = {
          enable = true;
          autodetect = true;
        };
        earlyoom = {
          enable = true;
          freeMemThreshold = 5;
        };
        sanoid = {
          enable = true;
          interval = "hourly";
          datasets."tank" = {
            recursive = true;
            autosnap = true;
            autoprune = true;
            hourly = 24;
            daily = 30;
            monthly = 3;
            yearly = 0;
          };
        };
      };

      zramSwap.enable = true;

      # Computed at boot so it stays correct regardless of installed RAM.
      systemd.services.zfs-arc-max = {
        description = "Cap ZFS ARC size to max(RAM - 1GiB, 5/8 * RAM)";
        after = [ "zfs-import.target" ];
        wantedBy = [ "zfs-import.target" ];
        unitConfig.ConditionPathExists = "/sys/module/zfs/parameters/zfs_arc_max";
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          ExecStart = pkgs.writeShellScript "zfs-arc-max" ''
            mem_bytes=$(${pkgs.gawk}/bin/awk '/^MemTotal:/ { print $2 * 1024 }' /proc/meminfo)
            reserve_1gib=$(( mem_bytes - 1073741824 ))
            five_eighths=$(( mem_bytes * 5 / 8 ))
            arc_max=$reserve_1gib
            if [ "$five_eighths" -gt "$reserve_1gib" ]; then
              arc_max=$five_eighths
            fi
            echo "$arc_max" > /sys/module/zfs/parameters/zfs_arc_max
          '';
        };
      };
    };
}

{
  config,
  ...
}:
{
  nixosModules.desktop.mount = {
    enable = true;
    mounts = {
      "jotunheim-homes" = {
        enable = true;
        type = "smb";
        server = "jotunheim.srv.plexuz.xyz";
        share = "homes";
        mountPoint = "/mnt/home";
        credentialsFile = config.sops.templates."jotunheim_homes_creds".path;
      };
      "jotunheim-kopia" = {
        enable = true;
        type = "nfs";
        server = "jotunheim.srv.plexuz.xyz";
        share = "/mnt/tank/kopia";
        mountPoint = "/mnt/kopia";
        nfsVersion = "4.2";
        autoMount = true;
        uid = 1000;
        gid = 100;
        extraOptions = [
          "proto=tcp"
          "rsize=8192"
          "wsize=8192"
        ];
      };
    };
  };
}

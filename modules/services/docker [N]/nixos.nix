{
  flake.modules.nixos.docker = {
    virtualisation.docker = {
      enable = true;
      daemon.settings = {
        log-driver = "journald";
        registry-mirrors = ["https://mirror.gcr.io"];
        storage-driver = "overlay2";
      };
      rootless = {
        enable = true;
        setSocketVariable = true;
      };
    };
  };
}

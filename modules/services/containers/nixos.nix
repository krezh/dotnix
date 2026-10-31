{
  flake.modules.nixos.containers = _: {
    virtualisation.docker = {
      enable = false;
      daemon.settings = {
        log-driver = "journald";
        registry-mirrors = [ "https://mirror.gcr.io" ];
        storage-driver = "overlay2";
      };
      rootless = {
        enable = true;
        setSocketVariable = true;
      };
    };
    virtualisation.podman = {
      enable = true;
      dockerCompat = true;
    };
  };
}

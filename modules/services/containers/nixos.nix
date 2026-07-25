{
  flake.modules.nixos.containers = _: {
    virtualisation.docker = {
      enable = true;
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
      enable = false;
      dockerCompat = true;
      dockerSocket.enable = true;
      autoPrune.enable = true;
    };
  };
}

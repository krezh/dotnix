# This file (and the global directory) holds config that i use on all hosts
{ ... }:
{
  virtualisation.docker = {
    enable = true;
    daemon.settings = {
      log-driver = "journald";
      registry-mirrors = [ "https://mirror.gcr.io" ];
      storage-driver = "overlay2";
    };
    # Use the rootless mode - run Docker daemon as non-root user
    rootless = {
      enable = true;
      setSocketVariable = true;
    };
  };
}

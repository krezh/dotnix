{
  config,
  pkgs,
  ...
}:
{
  xdg = {
    desktopEntries.sops = {
      name = "SOPS";
      exec = "${pkgs.sops}/bin/sops %F";
      mimeType = [ "application/x-sops-yaml" ];
      noDisplay = true;
    };
    mimeApps.defaultApplications."application/x-sops-yaml" = "sops.desktop";
  };

  home.file.".local/share/mime/packages/sops.xml".text = ''
    <?xml version="1.0" encoding="UTF-8"?>
    <mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
      <mime-type type="application/x-sops-yaml">
        <comment>SOPS encrypted YAML file</comment>
        <glob pattern="*.sops.yaml"/>
        <glob pattern="*.sops.yml"/>
      </mime-type>
    </mime-info>
  '';

  sops = {
    age.keyFile = "${config.xdg.configHome}/sops/age/keys.txt";
    defaultSopsFile = ./secrets.sops.yaml;
    secrets = {
      "ssh/privkey" = {
        path = "/home/${config.home.username}/.ssh/id_ed25519";
        mode = "0600";
      };
      "atuin/key" = {
        path = "${config.xdg.configHome}/atuin/key";
      };
      "zipline/token" = {
        path = "${config.xdg.configHome}/zipline/token";
      };
      "kopia/password" = {
        path = "${config.xdg.configHome}/kopia/repository.password";
      };
      "github/token" = {
        path = "${config.xdg.configHome}/github/token";
      };
    };
  };
  home = {
    sessionVariables = {
      SOPS_AGE_KEY_FILE = "${config.sops.age.keyFile}";
      SOPS_AGE_KEY_CMD = "age-plugin-yubikey --identity";
    };
  };
}

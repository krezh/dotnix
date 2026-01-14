{ inputs, ... }:
{
  flake.modules.nixos.thor = {
    home-manager.users.krezh =
      { config, ... }:
      {
        imports = [ inputs.self.modules.homeManager.kopia ];
        services.kopia = {
          enable = true;
          repository = {
            type = "filesystem";
            path = "/mnt/kopia";
            passwordFile = config.sops.secrets."kopia/password".path;
          };
          backups = {
            downloads = {
              paths = [ "${config.home.homeDirectory}/Downloads" ];
              schedule = "daily";
            };
            obsidian = {
              paths = [ "${config.home.homeDirectory}/Obsidian" ];
              schedule = "daily";
            };
            wow = {
              paths = [
                "${config.home.homeDirectory}/Games/Faugus/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/Interface"
                "${config.home.homeDirectory}/Games/Faugus/battlenet/drive_c/Program Files (x86)/World of Warcraft/_retail_/WTF"
              ];
              schedule = "daily";
            };
          };
        };
      };
  };
}

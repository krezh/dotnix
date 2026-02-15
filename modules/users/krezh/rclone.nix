{
  flake.modules.homeManager.krezh =
    { config, ... }:
    {
      programs.rclone = {
        enable = true;
        remotes = {
          garage = {
            config = {
              type = "s3";
              provider = "Other";
              endpoint = "https://s3.int.plexuz.xyz";
              region = "garage";
            };
            secrets = {
              access_key_id = config.sops.secrets."garage/accessID".path;
              secret_access_key = config.sops.secrets."garage/accessSecret".path;
            };
          };
          r2 = {
            config = {
              type = "s3";
              provider = "Other";
              endpoint = "520cf607cd37fea3cd09cd71aa255f6d.eu.r2.cloudflarestorage.com";
              region = "auto";
            };
            secrets = {
              access_key_id = config.sops.secrets."r2/accessID".path;
              secret_access_key = config.sops.secrets."r2/accessSecret".path;
            };
          };
        };
      };
    };
}

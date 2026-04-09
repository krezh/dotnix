{ ... }:
{
  flake.modules.homeManager.editors =
    { ... }:
    {
      programs.vscode = {
        profiles.default = {
          userSettings = {
            json.schemaDownload.enable = true;
            json.validate.enable = false;
            json5.schemaDownload.enable = true;
            json5.validate.enable = false;
          };
        };
      };
    };
}

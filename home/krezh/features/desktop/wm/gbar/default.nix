{ inputs, ... }: {
  imports = [ inputs.gBar.homeManagerModules.x86_64-linux.default ];

  programs.gBar = {
    enable = true;
    config = {
      Location = "L";
      EnableSNI = true;
      SNIIconSize = {
        Discord = 26;
        OBS = 23;
      };
      WorkspaceSymbols = [ " " " " ];
    };
  };
}

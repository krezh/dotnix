{ inputs, pkgs, ... }: {
  imports = [ inputs.gBar.homeManagerModules.${pkgs.system}.default ];

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

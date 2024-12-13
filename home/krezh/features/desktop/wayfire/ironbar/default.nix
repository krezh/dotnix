{ inputs, ... }:
{
  imports = [ inputs.ironbar.homeManagerModules.default ];

  programs.ironbar = {
    enable = true;
    config = { };
    style = "";
    features = [ ];
  };
}

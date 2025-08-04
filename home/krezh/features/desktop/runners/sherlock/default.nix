{ inputs, ... }:
{
  imports = [ inputs.sherlock.homeModules.default ];

  programs.sherlock.enable = true;
}

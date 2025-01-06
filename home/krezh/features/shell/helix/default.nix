{ pkgs, lib, ... }:
{
  imports = [ ];
  programs.helix = {
    enable = true;
    defaultEditor = true;
    languages = {
      language = [
        {
          name = "nix";
          auto-format = true;
          formatter.command = "${lib.getExe pkgs.nixfmt-rfc-style}";
        }
      ];
    };
  };
}

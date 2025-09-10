{ pkgs, ... }:
{
  hmModules.desktop.discord.enable = false;
  hmModules.desktop.discord.package = pkgs.legcord;
}

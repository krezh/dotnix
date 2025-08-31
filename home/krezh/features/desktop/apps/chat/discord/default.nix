{ pkgs, ... }:
{
  hmModules.desktop.discord.enable = true;
  hmModules.desktop.discord.package = pkgs.legcord;
}

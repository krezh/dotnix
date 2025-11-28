{ pkgs, ... }:
{
  home.packages = with pkgs; [
    capacities
  ];
}

{ pkgs, ... }: {
  services.clipman = {
    enable = true;
    package = pkgs.clipman;
  };
  home.packages = with pkgs; [ clipman ];
}

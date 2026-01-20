{
  flake.modules.homeManager.ai = {pkgs, ...}: {
    home.packages = [pkgs.github-copilot-cli];
  };
}

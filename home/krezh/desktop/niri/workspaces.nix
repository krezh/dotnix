{ ... }:
{
  programs.niri.settings = {
    workspaces = {
      "a-1" = {
        name = "1";
        open-on-output = "DP-1";
      };

      "a-2" = {
        name = "2";
        open-on-output = "DP-1";
      };

      "a-3" = {
        name = "3";
        open-on-output = "DP-1";
      };

      "a-4" = {
        name = "4";
        open-on-output = "DP-2";
      };

      "a-5" = {
        name = "5";
        open-on-output = "DP-2";
      };

      "a-6" = {
        name = "6";
        open-on-output = "DP-2";
      };
    };
  };
}

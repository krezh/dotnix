{ inputs, ... }:
{
  imports = [ inputs.walker.homeManagerModules.default ];

  programs.walker = {
    enable = true;
    runAsService = true;
    #   config = {
    #     providers.prefixes = [
    #       {
    #         provider = "websearch";
    #         prefix = "+";
    #       }
    #       {
    #         provider = "providerlist";
    #         prefix = "_";
    #       }
    #     ];
    #     # keybinds.quick_activate = [
    #     #   "1"
    #     #   "2"
    #     #   "3"
    #     # ];
    #   };

    #   # If this is not set the default styling is used.
    #   # theme.style = ''
    #   #   * {
    #   #     color: #dcd7ba;
    #   #   }
    #   # '';
  };
}

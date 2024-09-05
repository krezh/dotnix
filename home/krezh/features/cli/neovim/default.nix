{ inputs, ... }:
{
  imports = [ inputs.nixvim.homeManagerModules.nixvim ];
  programs.nixvim = {
    #package = inputs.nixvim.packages.${pkgs.system}.default;
    enable = false;
    viAlias = true;
    vimAlias = true;
    defaultEditor = true;
    plugins = {
      lightline.enable = true;
      treesitter.enable = true;
      telescope.enable = true;
      oil.enable = true;
      lsp = {
        enable = true;
        servers = {
          lua-ls.enable = true;
          gopls.enable = true;
        };
      };
    };
    colorschemes.catppuccin.enable = true;
    # extraPlugins = with pkgs.vimPlugins; [
    #   vim-nix
    # ];
    opts = {
      number = true; # Show line numbers
      relativenumber = true; # Show relative line numbers
      shiftwidth = 2; # Tab width should be 2
    };
  };
}

{ inputs, ... }:
{
  flake.modules.homeManager.editors.imports = [ inputs.nvf.homeManagerModules.default ];

  flake.modules.homeManager.editors.programs.nvf = {
    enable = true;
    defaultEditor = true;
    settings = {
      vim = {
        viAlias = true;
        vimAlias = true;
        languages = {
          enableFormat = false;
          enableTreesitter = false;
          enableExtraDiagnostics = false;
          nix.enable = true;
          markdown.enable = true;
          go.enable = true;
          bash.enable = true;
        };
        visuals = {
          nvim-web-devicons.enable = true;
          nvim-scrollbar.enable = true;
          cinnamon-nvim.enable = true;
          cellular-automaton.enable = false;
          fidget-nvim.enable = true;
          highlight-undo.enable = true;
          indent-blankline.enable = true;
          nvim-cursorline = {
            enable = true;
            setupOpts.line_timeout = 0;
          };
        };
        statusline.lualine = {
          enable = true;
          theme = "catppuccin";
        };
        theme = {
          enable = true;
          name = "catppuccin";
          style = "mocha";
          transparent = false;
        };
        autopairs.nvim-autopairs.enable = true;
        autocomplete.nvim-cmp.enable = true;
        tabline.nvimBufferline.enable = true;
        treesitter.context.enable = false;
        binds = {
          whichKey.enable = true;
          cheatsheet.enable = true;
        };
        telescope.enable = true;
        git = {
          enable = true;
          gitsigns.enable = true;
          gitsigns.codeActions.enable = false;
        };
        dashboard = {
          dashboard-nvim.enable = false;
          alpha.enable = true;
        };
        notify.nvim-notify.enable = true;
        ui = {
          borders.enable = true;
          noice.enable = true;
          colorizer.enable = true;
          modes-nvim.enable = false;
          illuminate.enable = true;
          breadcrumbs = {
            enable = true;
            navbuddy.enable = true;
          };
          fastaction.enable = true;
        };
        comments.comment-nvim.enable = true;
      };
    };
  };
}

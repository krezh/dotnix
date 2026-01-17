{
  flake.modules.homeManager.terminal = {
    catppuccin.wezterm.apply = false;
    catppuccin.wezterm.enable = false;
    programs.wezterm = {
      enable = true;
      extraConfig = ''
        local wezterm = require 'wezterm'
        local act = wezterm.action

        local workspace_switcher = wezterm.plugin.require('https://github.com/MLFlexer/smart_workspace_switcher.wezterm')

        local config = {}
        if wezterm.config_builder then
          config = wezterm.config_builder()
        end

        config.check_for_updates = false

        config.color_scheme = 'Catppuccin Mocha'

        config.font = wezterm.font_with_fallback {
          {
            family = 'JetBrainsMono Nerd Font',
            harfbuzz_features = { 'calt=0', 'dlig=1', },
          },
          'Symbols Nerd Font',
          'Flog Symbols',
        }
        config.font_size = 11
        config.warn_about_missing_glyphs = false

        config.use_fancy_tab_bar = false
        config.enable_tab_bar = false

        config.inactive_pane_hsb = {
          saturation = 0.9,
          brightness = 0.7,
        }

        -- Show which key table is active in the status area
        wezterm.on('update-right-status', function(window, pane)
          local key_table = window:active_key_table()
          if key_table then
            key_table = 'TABLE: ' .. key_table .. ' | '
          end

          window:set_right_status((key_table or ''') .. window:active_workspace())
        end)

        config.leader = {
          key = 'Space',
          mods = 'CTRL',
          timeout_milliseconds = math.maxinteger,
        }

        config.window_decorations = 'NONE'

        config.pane_focus_follows_mouse = false
        config.mouse_wheel_scrolls_tabs = false

        config.default_workspace = '~'

        config.disable_default_key_bindings = true
        config.keys = {
          { key = 'Enter',      mods = 'CTRL|SHIFT',   action = act.ActivateKeyTable { name = 'as_is', }, },

          { key = 'n',          mods = 'LEADER',       action = act.ActivateTabRelative(1), },
          { key = 'p',          mods = 'LEADER',       action = act.ActivateTabRelative(-1), },

          { key = 'n',          mods = 'LEADER|SHIFT', action = act.ActivateTab(-1), },
          { key = 'p',          mods = 'LEADER|SHIFT', action = act.ActivateTab(0), },

          { key = '[',          mods = 'LEADER',       action = act.MoveTabRelative(-1), },
          { key = ']',          mods = 'LEADER',       action = act.MoveTabRelative(1), },

          { key = '1',          mods = 'LEADER',       action = act.ActivateTab(0), },
          { key = '2',          mods = 'LEADER',       action = act.ActivateTab(1), },
          { key = '3',          mods = 'LEADER',       action = act.ActivateTab(2), },
          { key = '4',          mods = 'LEADER',       action = act.ActivateTab(3), },
          { key = '5',          mods = 'LEADER',       action = act.ActivateTab(4), },
          { key = '6',          mods = 'LEADER',       action = act.ActivateTab(5), },
          { key = '7',          mods = 'LEADER',       action = act.ActivateTab(6), },
          { key = '8',          mods = 'LEADER',       action = act.ActivateTab(7), },
          { key = '9',          mods = 'LEADER',       action = act.ActivateTab(8), },
          { key = '0',          mods = 'LEADER',       action = act.ActivateTab(9), },

          { key = '1',          mods = 'LEADER|SHIFT', action = act.MoveTab(0), },
          { key = '2',          mods = 'LEADER|SHIFT', action = act.MoveTab(1), },
          { key = '3',          mods = 'LEADER|SHIFT', action = act.MoveTab(2), },
          { key = '4',          mods = 'LEADER|SHIFT', action = act.MoveTab(3), },
          { key = '5',          mods = 'LEADER|SHIFT', action = act.MoveTab(4), },
          { key = '6',          mods = 'LEADER|SHIFT', action = act.MoveTab(5), },
          { key = '7',          mods = 'LEADER|SHIFT', action = act.MoveTab(6), },
          { key = '8',          mods = 'LEADER|SHIFT', action = act.MoveTab(7), },
          { key = '9',          mods = 'LEADER|SHIFT', action = act.MoveTab(8), },
          { key = '0',          mods = 'LEADER|SHIFT', action = act.MoveTab(9), },

          { key = 'LeftArrow',  mods = 'SHIFT|CTRL',   action = act.ActivatePaneDirection 'Left', },
          { key = 'RightArrow', mods = 'SHIFT|CTRL',   action = act.ActivatePaneDirection 'Right', },
          { key = 'UpArrow',    mods = 'SHIFT|CTRL',   action = act.ActivatePaneDirection 'Up', },
          { key = 'DownArrow',  mods = 'SHIFT|CTRL',   action = act.ActivatePaneDirection 'Down', },

          { key = 'h',          mods = 'LEADER',       action = act.ActivatePaneDirection 'Left', },
          { key = 'k',          mods = 'LEADER',       action = act.ActivatePaneDirection 'Up', },
          { key = 'j',          mods = 'LEADER',       action = act.ActivatePaneDirection 'Down', },
          { key = 'l',          mods = 'LEADER',       action = act.ActivatePaneDirection 'Right', },
          { key = 'w',          mods = 'LEADER',       action = act.PaneSelect { alphabet = 'tnseriaodh', }, },

          { key = 's',          mods = 'LEADER',       action = act.SplitVertical { domain = 'CurrentPaneDomain', }, },
          { key = 'v',          mods = 'LEADER',       action = act.SplitHorizontal { domain = 'CurrentPaneDomain', }, },
          { key = 'x',          mods = 'LEADER',       action = act.CloseCurrentPane { confirm = true, }, },

          { key = 'c',          mods = 'LEADER',       action = act.SpawnTab 'CurrentPaneDomain', },
          { key = 't',          mods = 'SHIFT|CTRL',   action = act.SpawnTab 'CurrentPaneDomain', },
          { key = 't',          mods = 'SUPER',        action = act.SpawnTab 'CurrentPaneDomain', },

          { key = 'q',          mods = 'LEADER',       action = act.CloseCurrentTab { confirm = true, }, },
          { key = 'w',          mods = 'SHIFT|CTRL',   action = act.CloseCurrentTab { confirm = true, }, },
          { key = 'w',          mods = 'SUPER',        action = act.CloseCurrentTab { confirm = true, }, },

          { key = 'v',          mods = 'LEADER|CTRL',  action = act.QuickSelect, },
          { key = 'x',          mods = 'LEADER|CTRL',  action = act.ActivateCopyMode, },

          { key = 'r',          mods = 'LEADER',       action = act.ActivateKeyTable { name = 'resize_pane', one_shot = false, }, },

          { key = 'f',          mods = 'LEADER',       action = workspace_switcher.switch_workspace(), },
          { key = 'f',          mods = 'LEADER|SHIFT', action = workspace_switcher.switch_to_prev_workspace(), },

          { key = 'Space',      mods = 'LEADER|CTRL',  action = act.ActivateCommandPalette, },
          { key = 'l',          mods = 'LEADER|CTRL',  action = act.ShowDebugOverlay, },
          { key = 'r',          mods = 'LEADER|CTRL',  action = act.ReloadConfiguration, },

          { key = ')',          mods = 'SHIFT|CTRL',   action = act.ResetFontSize, },
          { key = '0',          mods = 'SUPER',        action = act.ResetFontSize, },
          { key = '+',          mods = 'SHIFT|CTRL',   action = act.IncreaseFontSize, },
          { key = '=',          mods = 'SUPER',        action = act.IncreaseFontSize, },
          { key = '_',          mods = 'SHIFT|CTRL',   action = act.DecreaseFontSize, },
          { key = '-',          mods = 'SUPER',        action = act.DecreaseFontSize, },

          { key = 'c',          mods = 'SHIFT|CTRL',   action = act.CopyTo 'Clipboard', },
          { key = 'c',          mods = 'SUPER',        action = act.CopyTo 'Clipboard', },
          { key = 'Copy',       mods = 'NONE',         action = act.CopyTo 'Clipboard', },

          { key = 'v',          mods = 'SHIFT|CTRL',   action = act.PasteFrom 'Clipboard', },
          { key = 'v',          mods = 'SUPER',        action = act.PasteFrom 'Clipboard', },
          { key = 'Paste',      mods = 'NONE',         action = act.PasteFrom 'Clipboard', },

          { key = 'f',          mods = 'SHIFT|CTRL',   action = act.Search 'CurrentSelectionOrEmptyString', },
          { key = 'f',          mods = 'SUPER',        action = act.Search 'CurrentSelectionOrEmptyString', },

          { key = 'k',          mods = 'SHIFT|CTRL',   action = act.ClearScrollback 'ScrollbackOnly', },
          { key = 'k',          mods = 'SUPER',        action = act.ClearScrollback 'ScrollbackOnly', },

          { key = 'n',          mods = 'SHIFT|CTRL',   action = act.SpawnWindow, },
          { key = 'n',          mods = 'SUPER',        action = act.SpawnWindow, },

          { key = 'q',          mods = 'SHIFT|CTRL',   action = act.QuitApplication, },
          { key = 'q',          mods = 'SUPER',        action = act.Nop, },

          -- { key = "h",          mods = "SHIFT|CTRL",   action = act.HideApplication },
          -- { key = "h",          mods = "SUPER",        action = act.HideApplication },

          -- { key = "m",          mods = "SHIFT|CTRL",   action = act.Hide },
          -- { key = "m",          mods = "SUPER",        action = act.Hide },

          { key = 'z',          mods = 'SHIFT|CTRL',   action = act.TogglePaneZoomState, },

          { key = 'PageUp',     mods = 'SHIFT',        action = act.ScrollByPage(-1), },
          { key = 'PageDown',   mods = 'SHIFT',        action = act.ScrollByPage(1), },
        }

        config.key_tables = {
          resize_pane = {
            { key = 'Escape',     action = 'PopKeyTable', },
            { key = 'c',          mods = 'CTRL',                               action = 'PopKeyTable', },
            { key = '[',          mods = 'CTRL',                               action = 'PopKeyTable', },

            { key = 'LeftArrow',  action = act.AdjustPaneSize { 'Left', 5, }, },
            { key = 'h',          action = act.AdjustPaneSize { 'Left', 5, }, },

            { key = 'RightArrow', action = act.AdjustPaneSize { 'Right', 5, }, },
            { key = 'l',          action = act.AdjustPaneSize { 'Right', 5, }, },

            { key = 'UpArrow',    action = act.AdjustPaneSize { 'Up', 5, }, },
            { key = 'k',          action = act.AdjustPaneSize { 'Up', 5, }, },

            { key = 'DownArrow',  action = act.AdjustPaneSize { 'Down', 5, }, },
            { key = 'j',          action = act.AdjustPaneSize { 'Down', 5, }, },
          },

          copy_mode = {
            { key = 'Escape', mods = 'NONE',  action = act.CopyMode 'Close', },
            { key = 'c',      mods = 'CTRL',  action = act.CopyMode 'Close', },
            { key = '[',      mods = 'CTRL',  action = act.CopyMode 'Close', },

            { key = 'Tab',    mods = 'NONE',  action = act.CopyMode 'MoveForwardWord', },
            { key = 'Tab',    mods = 'SHIFT', action = act.CopyMode 'MoveBackwardWord', },
            { key = 'Enter',  mods = 'NONE',  action = act.CopyMode 'MoveToStartOfNextLine', },
            { key = 'Space',  mods = 'NONE',  action = act.CopyMode { SetSelectionMode = 'Cell', }, },
            { key = '$',      mods = 'NONE',  action = act.CopyMode 'MoveToEndOfLineContent', },
            { key = '$',      mods = 'SHIFT', action = act.CopyMode 'MoveToEndOfLineContent', },
            { key = ',',      mods = 'NONE',  action = act.CopyMode 'JumpReverse', },
            { key = '0',      mods = 'NONE',  action = act.CopyMode 'MoveToStartOfLine', },
            { key = ';',      mods = 'NONE',  action = act.CopyMode 'JumpAgain', },
            { key = 'F',      mods = 'NONE',  action = act.CopyMode { JumpBackward = { prev_char = false, }, }, },
            { key = 'F',      mods = 'SHIFT', action = act.CopyMode { JumpBackward = { prev_char = false, }, }, },
            { key = 'G',      mods = 'NONE',  action = act.CopyMode 'MoveToScrollbackBottom', },
            { key = 'G',      mods = 'SHIFT', action = act.CopyMode 'MoveToScrollbackBottom', },
            { key = 'H',      mods = 'NONE',  action = act.CopyMode 'MoveToViewportTop', },
            { key = 'H',      mods = 'SHIFT', action = act.CopyMode 'MoveToViewportTop', },
            { key = 'L',      mods = 'NONE',  action = act.CopyMode 'MoveToViewportBottom', },
            { key = 'L',      mods = 'SHIFT', action = act.CopyMode 'MoveToViewportBottom', },
            { key = 'M',      mods = 'NONE',  action = act.CopyMode 'MoveToViewportMiddle', },
            { key = 'M',      mods = 'SHIFT', action = act.CopyMode 'MoveToViewportMiddle', },
            { key = 'O',      mods = 'NONE',  action = act.CopyMode 'MoveToSelectionOtherEndHoriz', },
            { key = 'O',      mods = 'SHIFT', action = act.CopyMode 'MoveToSelectionOtherEndHoriz', },
            { key = 'T',      mods = 'NONE',  action = act.CopyMode { JumpBackward = { prev_char = true, }, }, },
            { key = 'T',      mods = 'SHIFT', action = act.CopyMode { JumpBackward = { prev_char = true, }, }, },
            { key = 'V',      mods = 'NONE',  action = act.CopyMode { SetSelectionMode = 'Line', }, },
            { key = 'V',      mods = 'SHIFT', action = act.CopyMode { SetSelectionMode = 'Line', }, },
            { key = '^',      mods = 'NONE',  action = act.CopyMode 'MoveToStartOfLineContent', },
            { key = '^',      mods = 'SHIFT', action = act.CopyMode 'MoveToStartOfLineContent', },
            { key = 'b',      mods = 'NONE',  action = act.CopyMode 'MoveBackwardWord', },
            { key = 'b',      mods = 'ALT',   action = act.CopyMode 'MoveBackwardWord', },
            { key = 'b',      mods = 'CTRL',  action = act.CopyMode 'PageUp', },
            { key = 'c',      mods = 'CTRL',  action = act.CopyMode 'Close', },
            { key = 'd',      mods = 'CTRL',  action = act.CopyMode { MoveByPage = 0.5, }, },
            { key = 'e',      mods = 'NONE',  action = act.CopyMode 'MoveForwardWordEnd', },
            { key = 'f',      mods = 'NONE',  action = act.CopyMode { JumpForward = { prev_char = false, }, }, },
            { key = 'f',      mods = 'ALT',   action = act.CopyMode 'MoveForwardWord', },
            { key = 'f',      mods = 'CTRL',  action = act.CopyMode 'PageDown', },
            { key = 'g',      mods = 'NONE',  action = act.CopyMode 'MoveToScrollbackTop', },
            { key = 'g',      mods = 'CTRL',  action = act.CopyMode 'Close', },
            { key = 'h',      mods = 'NONE',  action = act.CopyMode 'MoveLeft', },
            { key = 'j',      mods = 'NONE',  action = act.CopyMode 'MoveDown', },
            { key = 'k',      mods = 'NONE',  action = act.CopyMode 'MoveUp', },
            { key = 'l',      mods = 'NONE',  action = act.CopyMode 'MoveRight', },
            { key = 'm',      mods = 'ALT',   action = act.CopyMode 'MoveToStartOfLineContent', },
            { key = 'o',      mods = 'NONE',  action = act.CopyMode 'MoveToSelectionOtherEnd', },
            { key = 'q',      mods = 'NONE',  action = act.CopyMode 'Close', },
            { key = 't',      mods = 'NONE',  action = act.CopyMode { JumpForward = { prev_char = true, }, }, },
            { key = 'u',      mods = 'CTRL',  action = act.CopyMode { MoveByPage = -0.5, }, },
            { key = 'v',      mods = 'NONE',  action = act.CopyMode { SetSelectionMode = 'Cell', }, },
            { key = 'v',      mods = 'CTRL',  action = act.CopyMode { SetSelectionMode = 'Block', }, },
            { key = 'w',      mods = 'NONE',  action = act.CopyMode 'MoveForwardWord', },
            {
              key = 'y',
              mods = 'NONE',
              action = act.Multiple { act.CopyTo 'ClipboardAndPrimarySelection', act.CopyMode 'Close', },
            },
            { key = 'PageUp',     mods = 'NONE', action = act.CopyMode 'PageUp', },
            { key = 'PageDown',   mods = 'NONE', action = act.CopyMode 'PageDown', },
            { key = 'End',        mods = 'NONE', action = act.CopyMode 'MoveToEndOfLineContent', },
            { key = 'Home',       mods = 'NONE', action = act.CopyMode 'MoveToStartOfLine', },
            { key = 'LeftArrow',  mods = 'NONE', action = act.CopyMode 'MoveLeft', },
            { key = 'LeftArrow',  mods = 'ALT',  action = act.CopyMode 'MoveBackwardWord', },
            { key = 'RightArrow', mods = 'NONE', action = act.CopyMode 'MoveRight', },
            { key = 'RightArrow', mods = 'ALT',  action = act.CopyMode 'MoveForwardWord', },
            { key = 'UpArrow',    mods = 'NONE', action = act.CopyMode 'MoveUp', },
            { key = 'DownArrow',  mods = 'NONE', action = act.CopyMode 'MoveDown', },
          },

          search_mode = {
            { key = 'Escape',    mods = 'NONE', action = act.CopyMode 'Close', },
            { key = 'c',         mods = 'CTRL', action = act.CopyMode 'Close', },
            { key = '[',         mods = 'CTRL', action = act.CopyMode 'Close', },

            { key = 'Enter',     mods = 'NONE', action = act.CopyMode 'PriorMatch', },
            { key = 'n',         mods = 'CTRL', action = act.CopyMode 'NextMatch', },
            { key = 'p',         mods = 'CTRL', action = act.CopyMode 'PriorMatch', },
            { key = 'r',         mods = 'CTRL', action = act.CopyMode 'CycleMatchType', },
            { key = 'u',         mods = 'CTRL', action = act.CopyMode 'ClearPattern', },
            { key = 'PageUp',    mods = 'NONE', action = act.CopyMode 'PriorMatchPage', },
            { key = 'PageDown',  mods = 'NONE', action = act.CopyMode 'NextMatchPage', },
            { key = 'UpArrow',   mods = 'NONE', action = act.CopyMode 'PriorMatch', },
            { key = 'DownArrow', mods = 'NONE', action = act.CopyMode 'NextMatch', },
          },

          as_is = {
            { key = 'Space', mods = 'CTRL', action = act.SendKey { key = 'Space', mods = 'CTRL', }, },
            { key = 'h',     mods = 'CTRL', action = act.SendKey { key = 'h', mods = 'CTRL', }, },
            { key = 'j',     mods = 'CTRL', action = act.SendKey { key = 'j', mods = 'CTRL', }, },
            { key = 'k',     mods = 'CTRL', action = act.SendKey { key = 'k', mods = 'CTRL', }, },
            { key = 'l',     mods = 'CTRL', action = act.SendKey { key = 'l', mods = 'CTRL', }, },
          },
        }

        wezterm.plugin.update_all()

        return config

      '';
    };
  };
}

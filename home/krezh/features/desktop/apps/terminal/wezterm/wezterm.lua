local wezterm = require("wezterm")
local act = wezterm.action
local config = wezterm.config_builder()
local is_windows = wezterm.target_triple == "x86_64-pc-windows-msvc"

if is_windows then
  config.default_domain = "WSL:NixOS"
  config.keys = {
  {
    key = '9',
    mods = 'ALT',
    action = wezterm.action.ShowLauncherArgs { flags = 'LAUNCH_MENU_ITEMS' },
  },
  }
  config.launch_menu = {
    {
      label = 'NixOS',
      domain = {DomainName = 'WSL:NixOS' },
    },
    {
      label = 'Arch',
      domain = {DomainName = 'WSL:Arch' },
    },
  }

end

--for _, gpu in ipairs(wezterm.gui.enumerate_gpus()) do
--  if gpu.device_type == "DiscreteGpu" then
--    config.webgpu_preferred_adapter = gpu
--    config.webgpu_power_preference = "HighPerformance"
--    config.front_end = "WebGpu"
--    break
--  end
--end

-- Modify config properties directly
config.hide_tab_bar_if_only_one_tab = true
config.color_scheme = "tokyonight_night" --"Catppuccin Mocha"
config.font = wezterm.font("JetBrains Mono")
config.font_size = 13.0
config.macos_window_background_blur = 30
config.window_background_opacity = 60
config.win32_system_backdrop = "Mica"
config.max_fps = 120
config.animation_fps = 30
config.audible_bell = "Disabled"
config.initial_cols = 160
config.initial_rows = 40
config.use_fancy_tab_bar = false
config.window_close_confirmation = 'NeverPrompt'

-- Change Mouse Bindings
config.mouse_bindings = {
  -- Bind 'Up' event of CTRL-Click to open hyperlinks
  {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'CTRL',
    action = act.OpenLinkAtMouseCursor,
  },
  -- Disable the 'Down' event of CTRL-Click to avoid weird program behaviors
  {
    event = { Down = { streak = 1, button = 'Left' } },
    mods = 'CTRL',
    action = act.Nop,
  },
  {
    event = { Up = { streak = 1, button = 'Right' } },
    mods = 'SHIFT',
    action = act.PasteFrom('Clipboard'),
  },
  -- Change the default click behavior so that it only selects
  -- text and doesn't open hyperlinks
  {
    event = { Up = { streak = 1, button = 'Left' } },
    mods = 'NONE',
    action = act.CompleteSelection 'ClipboardAndPrimarySelection',
  },
}


return config

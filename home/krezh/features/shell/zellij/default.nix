{
  inputs,
  pkgs,
  osConfig,
  ...
}:
let
  zjstatus = inputs.zjstatus.packages.${pkgs.system}.default;
  room = builtins.fetchurl {
    url = "https://github.com/rvcas/room/releases/download/v1.2.0/room.wasm";
    sha256 = "0k5fy3svjvifsgp0kdvqdx9m9rzrql9cwq6hbvxdgklfnczqz8dp";
  };
in
{
  programs.zellij = {
    enable = false;
    enableFishIntegration = true;
  };

  xdg.configFile."zellij/layouts/default.kdl".text = ''
    simplified_ui false
    pane_frames false
    keybinds {
      shared_except "locked" {
        bind "Ctrl y" {
          LaunchOrFocusPlugin "file:${room}" {
            floating true
            ignore_case true
            quick_jump true
          }
        }
      }
      // keybinds are divided into modes
      normal {
        // bind instructions can include one or more keys (both keys will be bound separately)
        // bind keys can include one or more actions (all actions will be performed with no sequential guarantees)
        bind "Ctrl g" { SwitchToMode "locked"; }
        bind "Ctrl p" { SwitchToMode "pane"; }
        bind "Alt n" { NewPane; }
        bind "Alt t" { NewTab; }
        bind "Alt h" "Alt Left" { MoveFocusOrTab "Left"; }
      }
      pane {
        bind "h" "Left" { MoveFocus "Left"; }
        bind "l" "Right" { MoveFocus "Right"; }
        bind "j" "Down" { MoveFocus "Down"; }
        bind "k" "Up" { MoveFocus "Up"; }
        bind "p" { SwitchFocus; }
      }
      locked {
        bind "Ctrl g" { SwitchToMode "normal"; }
      }
    }

    layout {
      default_tab_template {
        children
        pane size=1 borderless=false {
          plugin location="file://${zjstatus}/bin/zjstatus.wasm" {
            format_left   "{mode}#[fg=black]{tabs}"
            format_center "" // "{session}"
            format_right  "#[fg=#6867AF,bold] {datetime}#[fg=cyan]#[fg=#6867AF,bold] "
            format_space  "#[fg=yellow] "
            hide_frame_for_single_pane "false"
            hide_frame_except_for_search  "true"
            border_enabled  "false"
            mode_normal  "#[fg=yellow]   "
            mode_locked  "#[fg=red]   "
            mode_tmux    "#[bg=cyan]   "
            // 
            // ●
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            // 
            //  
            // 
            // 
            // 
            // 
            // 
            // 
            // 󰀱
            // 󰚄
            // 󰝆
            // 󰡄
            // 󰨈
            // 󰩖
            // 󰩗
            // 󱂡
            // 󱂛
            // 󱂑
            // 󱂐
            // 󱂏
            // 󱂎
            // 󱂍
            // 󱂌
            // 󱂋
            // 󱂊
            // 󱂉
            // 󱂅
            // 󱂈
            // 󰟹
            // 󰟯
            // 󰟲
            // 󰟰
            // 󰟽
            // 󰎢
            // 󰎣
            // 󰎥
            // 󰎦
            // 󰎨
            // 󰎩
            // 󰎫
            // 󰎬
            // 󰎲
            // 󰎮
            // 󰎯
            // 󰎰
            // 󰎴
            // 󰎵
            // 󰎷
            // 󰎸
            // 󰎺
            // 󰎻
            // 󰎽
            // 󰎾   
            // mode_normal        "#[bg=#89B4FA] {name} "
            // mode_locked        "#[bg=#89B4FA] {name} "
            // mode_resize        "#[bg=#89B4FA] {name} "
            // mode_pane          "#[bg=#89B4FA] {name} "
            // mode_tab           "#[bg=#89B4FA] {name} "
            // mode_scroll        "#[bg=#89B4FA] {name} "
            // mode_enter_search  "#[bg=#89B4FA] {name} "
            // mode_search        "#[bg=#89B4FA] {name} "
            // mode_rename_tab    "#[bg=#89B4FA] {name} "
            // mode_rename_pane   "#[bg=#89B4FA] {name} "
            // mode_session       "#[bg=#89B4FA] {name} "
            // mode_move          "#[bg=#89B4FA] {name} "
            // mode_prompt        "#[bg=#89B4FA] {name} "
            // mode_tmux          "#[bg=#ffc387] {name} "
            // formatting for inactive tabs
            tab_normal              "#[fg=#6C7086]{name}"
            tab_normal_fullscreen   "#[fg=#6C7086]{name}"
            tab_normal_sync         "#[fg=#6C7086]{name}"
            // formatting for the current active tab
            tab_active              "#[fg=blue,bold]{name}#[fg=yellow,bold]{floating_indicator}"
            tab_active_fullscreen   "#[fg=yellow,bold]{name}#[fg=yellow,bold]{fullscreen_indicator}"
            tab_active_sync         "#[fg=green,bold]{name}#[fg=yellow,bold]{sync_indicator}"
            // separator between the tabs
            tab_separator           "#[fg=cyan,bold] ⋮ "
            // indicators
            tab_sync_indicator       " "
            tab_fullscreen_indicator " "
            tab_floating_indicator   ""
            datetime        "#[fg=#6C7086,bold] {format} "
            datetime_format "%A, %d %b %Y %H:%M"
            datetime_timezone "${osConfig.time.timeZone}"
          }
        }
      }
    }
  '';
}

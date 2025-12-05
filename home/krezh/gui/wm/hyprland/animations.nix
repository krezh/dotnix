{ ... }:
{
  wayland.windowManager.hyprland.settings = {
    animations = {
      enabled = true;
      bezier = [
        "easeOutExpo,0.16,1,0.3,1"
        "easeOutQuad,0.25,0.46,0.45,0.94"
        "spring,0.25,0.1,0.25,1"
      ];

      animation = [
        "windowsIn,1,3,spring"
        "windowsOut,1,2,easeOutQuad,popin 80%"
        "windowsMove,1,3,spring"
        "workspaces,1,3,spring,slidevert"
        "border,1,3,spring"
        "fade,1,1,spring"
        "layers,1,3,easeOutQuad"
      ];
    };
  };
}

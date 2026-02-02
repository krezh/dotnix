{
  lib,
  hyprland,
  hyprlandPlugins,
}:
hyprlandPlugins.mkHyprlandPlugin {
  pluginName = "hypr-ungrab";
  version = "1.0.0";
  src = ./.;

  inherit (hyprland) nativeBuildInputs;

  meta = with lib; {
    homepage = "https://github.com/krezh/nix-config";
    description = "Hyprland plugin to release pointer grab/constraints with a keybind";
    license = licenses.bsd3;
    platforms = platforms.linux;
  };
}

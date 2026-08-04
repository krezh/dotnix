#define WLR_USE_UNSTABLE

#include <hyprland/src/Compositor.hpp>

#include "globals.hpp"
#include "ScrollDrag.hpp"

APICALL EXPORT std::string PLUGIN_API_VERSION() {
    return HYPRLAND_API_VERSION;
}

APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE handle) {
    PHANDLE = handle;

    const std::string HASH        = __hyprland_api_get_hash();
    const std::string CLIENT_HASH = __hyprland_api_get_client_hash();

    if (HASH != CLIENT_HASH) {
        HyprlandAPI::addNotification(PHANDLE, "[scrolldrag] Failure in initialization: version mismatch (headers != running Hyprland)", CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[scrolldrag] Version mismatch");
    }

    g_pGlobalState = makeUnique<SGlobalState>();

    g_pGlobalState->config.sensitivity = makeShared<Config::Values::CFloatValue>("plugin:scrolldrag:sensitivity", "Multiplier applied to horizontal drag delta", 1.0F);
    g_pGlobalState->config.deadzone =
        makeShared<Config::Values::CIntValue>("plugin:scrolldrag:deadzone", "Pixels of movement before the drag axis locks", 8);
    g_pGlobalState->config.workspaceSwitchThreshold = makeShared<Config::Values::CIntValue>(
        "plugin:scrolldrag:workspace_switch_threshold", "Pixels of vertical drag needed to trigger a workspace switch", 300);

    HyprlandAPI::addConfigValueV2(PHANDLE, g_pGlobalState->config.sensitivity);
    HyprlandAPI::addConfigValueV2(PHANDLE, g_pGlobalState->config.deadzone);
    HyprlandAPI::addConfigValueV2(PHANDLE, g_pGlobalState->config.workspaceSwitchThreshold);

    g_pScrollDrag = makeUnique<CScrollDragGesture>();

    HyprlandAPI::addNotification(PHANDLE, "[scrolldrag] loaded", CHyprColor{0.2, 1.0, 0.2, 1.0}, 3000);

    return {"scrolldrag", "SUPER + middle-mouse drag pan/switch for the scrolling layout", "krezh", "0.1"};
}

APICALL EXPORT void PLUGIN_EXIT() {
    g_pScrollDrag.reset();
    g_pGlobalState.reset();
}

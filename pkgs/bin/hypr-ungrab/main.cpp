#define WLR_USE_UNSTABLE

#include <any>
#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/managers/input/InputManager.hpp>
#include <hyprland/src/protocols/PointerConstraints.hpp>
#include <hyprgraphics/color/Color.hpp>

#include "globals.hpp"

using namespace Hyprgraphics;

// Do NOT change this function.
APICALL EXPORT std::string PLUGIN_API_VERSION() {
    return HYPRLAND_API_VERSION;
}

// Dispatcher to release pointer grab
static void releasePointerGrab(std::string) {
    // Get all active constraints and deactivate them
    for (auto& wc : g_pInputManager->m_constraints) {
        auto constraint = wc.lock();
        if (!constraint || !constraint->isActive())
            continue;

        constraint->deactivate();
    }

    HyprlandAPI::addNotification(PHANDLE, "[hypr-ungrab] Released pointer grab", CHyprColor{0.2, 1.0, 0.2, 1.0}, 2000);
}

APICALL EXPORT PLUGIN_DESCRIPTION_INFO PLUGIN_INIT(HANDLE handle) {
    PHANDLE = handle;

    const std::string HASH = __hyprland_api_get_hash();
    const std::string CLIENT_HASH = __hyprland_api_get_client_hash();

    if (HASH != CLIENT_HASH) {
        HyprlandAPI::addNotification(PHANDLE, "[hypr-ungrab] Failure in initialization: Version mismatch (headers ver is not equal to running hyprland ver)",
        CHyprColor{1.0, 0.2, 0.2, 1.0}, 5000);
        throw std::runtime_error("[hypr-ungrab] Version mismatch");
    }

    // Register the dispatcher
    HyprlandAPI::addDispatcher(PHANDLE, "releasepointer", releasePointerGrab);

    HyprlandAPI::addNotification(PHANDLE, "[hypr-ungrab] Initialized successfully! Use 'releasepointer' dispatcher to release mouse grab.",
    CHyprColor{0.2, 1.0, 0.2, 1.0}, 5000);

    return {"hypr-ungrab", "Release pointer constraints", "krezh", "1.0"};
}

APICALL EXPORT void PLUGIN_EXIT() {
    HyprlandAPI::addNotification(PHANDLE, "[hypr-ungrab] Unloaded", CHyprColor{0.2, 1.0, 0.2, 1.0}, 5000);
}

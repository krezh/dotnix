#include "ScrollLayout.hpp"

#include <hyprland/src/Compositor.hpp>
#include <hyprland/src/desktop/state/FocusState.hpp>
#include <hyprland/src/desktop/Workspace.hpp>
#include <hyprland/src/output/Monitor.hpp>
#include <hyprland/src/layout/algorithm/Algorithm.hpp>
#include <hyprland/src/layout/algorithm/tiled/scrolling/ScrollingAlgorithm.hpp>
#include <hyprland/src/layout/space/Space.hpp>

Layout::Tiled::CScrollingAlgorithm* resolveActiveScrollingAlgorithm() {
    const auto MONITOR = Desktop::focusState()->monitor();
    if (!MONITOR)
        return nullptr;

    const auto WORKSPACE = MONITOR->m_activeSpecialWorkspace ? MONITOR->m_activeSpecialWorkspace : MONITOR->m_activeWorkspace;
    if (!WORKSPACE || !WORKSPACE->m_space)
        return nullptr;

    const auto ALGO = WORKSPACE->m_space->algorithm();
    if (!ALGO || !ALGO->tiledAlgo())
        return nullptr;

    return dynamic_cast<Layout::Tiled::CScrollingAlgorithm*>(ALGO->tiledAlgo().get());
}

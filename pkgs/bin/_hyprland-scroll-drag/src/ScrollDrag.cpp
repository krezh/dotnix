#include "ScrollDrag.hpp"

#include <linux/input-event-codes.h>

#include <hyprland/src/devices/IKeyboard.hpp>
#include <hyprland/src/layout/algorithm/tiled/scrolling/ScrollingAlgorithm.hpp>
#include <hyprland/src/managers/input/InputManager.hpp>
#include <hyprland/src/managers/KeybindManager.hpp>

#include "globals.hpp"
#include "ScrollLayout.hpp"

CScrollDragGesture::CScrollDragGesture() {
    m_mouseButtonListener = Event::bus()->m_events.input.mouse.button.listen([this](IPointer::SButtonEvent event, Event::SCallbackInfo& info) { onMouseButton(event, info); });
    m_mouseMoveListener   = Event::bus()->m_events.input.mouse.move.listen([this](Vector2D pos, Event::SCallbackInfo& info) { onMouseMove(pos, info); });
}

void CScrollDragGesture::reset() {
    m_dragging       = false;
    m_axis           = eDragAxis::AXIS_NONE;
    m_totalDelta     = {};
    m_lastTickDeltaX = 0.0;
    m_algorithm      = nullptr;
}

void CScrollDragGesture::onMouseButton(IPointer::SButtonEvent event, Event::SCallbackInfo& info) {
    const bool SUPER_HELD = (g_pInputManager->getModsFromAllKBs() & HL_MODIFIER_META) != 0;

    if (event.button == BTN_MIDDLE && event.state == WL_POINTER_BUTTON_STATE_PRESSED && SUPER_HELD && !m_dragging) {
        m_dragging = true;
        m_axis     = eDragAxis::AXIS_NONE;
        m_startPos = g_pInputManager->getMouseCoordsInternal();
        m_lastPos  = m_startPos;
        m_algorithm = resolveActiveScrollingAlgorithm();
        return;
    }

    if (event.button == BTN_MIDDLE && event.state == WL_POINTER_BUTTON_STATE_RELEASED && m_dragging) {
        const bool CROSSED_DEADZONE = m_axis != eDragAxis::AXIS_NONE;

        if (CROSSED_DEADZONE) {
            info.cancelled = true;

            if (m_axis == eDragAxis::AXIS_HORIZONTAL) {
                if (m_algorithm) {
                    // Project a few ticks of the last movement forward, so a fast flick lands on
                    // the next column instead of snapping straight back to the nearest one.
                    constexpr double PROJECTION_TICKS = 6.0;
                    const double     VIEWPORT          = m_algorithm->primaryViewportSize();
                    const double     PROJECTED         = VIEWPORT > 0 ? m_algorithm->normalizedTapeOffset() + (m_lastTickDeltaX * PROJECTION_TICKS) / VIEWPORT
                    : m_algorithm->normalizedTapeOffset();
                    m_algorithm->snapToProjectedOffset(PROJECTED);
                }
            } else {
                const int THRESHOLD = g_pGlobalState->config.workspaceSwitchThreshold->value();

                if (std::abs(m_totalDelta.y) >= THRESHOLD)
                    g_pKeybindManager->m_dispatchers["workspace"](m_totalDelta.y > 0 ? "+1" : "-1");
            }
        }

        reset();
        return;
    }
}

void CScrollDragGesture::onMouseMove(Vector2D pos, Event::SCallbackInfo& info) {
    if (!m_dragging)
        return;

    const Vector2D DELTA = pos - m_lastPos;
    m_lastPos             = pos;
    m_totalDelta          = pos - m_startPos;

    const int DEADZONE = g_pGlobalState->config.deadzone->value();

    if (m_axis == eDragAxis::AXIS_NONE) {
        if (m_totalDelta.size() < DEADZONE)
            return;

        m_axis = std::abs(m_totalDelta.x) >= std::abs(m_totalDelta.y) ? eDragAxis::AXIS_HORIZONTAL : eDragAxis::AXIS_VERTICAL;
        g_pInputManager->releaseAllMouseButtons();
    }

    info.cancelled = true;

    if (m_axis == eDragAxis::AXIS_HORIZONTAL) {
        if (m_algorithm) {
            const float SENSITIVITY = g_pGlobalState->config.sensitivity->value();
            const float SCALED      = -DELTA.x * SENSITIVITY;
            m_algorithm->moveTape(SCALED);
            m_lastTickDeltaX = SCALED; // used on release to project a flick past the current offset
        }
    }
    // AXIS_VERTICAL: no per-tick action, only accumulate m_totalDelta.y (already done above)
}

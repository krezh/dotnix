#pragma once

#include <hyprland/src/devices/IPointer.hpp>
#include <hyprland/src/event/EventBus.hpp>
#include <hyprland/src/helpers/math/Math.hpp>
#include <hyprland/src/helpers/memory/Memory.hpp>
#include <hyprland/src/helpers/signal/Signal.hpp>

namespace Layout::Tiled {
    class CScrollingAlgorithm;
}

enum class eDragAxis {
    AXIS_NONE = 0,
    AXIS_HORIZONTAL,
    AXIS_VERTICAL,
};

class CScrollDragGesture {
  public:
    CScrollDragGesture();
    ~CScrollDragGesture() = default;

  private:
    void onMouseButton(IPointer::SButtonEvent event, Event::SCallbackInfo& info);
    void onMouseMove(Vector2D pos, Event::SCallbackInfo& info);

    void reset();

    bool                                 m_dragging = false;
    eDragAxis                            m_axis     = eDragAxis::AXIS_NONE;
    Vector2D                             m_startPos;
    Vector2D                             m_lastPos;
    Vector2D                             m_totalDelta;

    Layout::Tiled::CScrollingAlgorithm*  m_algorithm      = nullptr;
    double                               m_lastTickDeltaX = 0.0;

    CHyprSignalListener                  m_mouseButtonListener;
    CHyprSignalListener                  m_mouseMoveListener;
};

inline UP<CScrollDragGesture> g_pScrollDrag;

#pragma once

namespace Layout::Tiled {
    class CScrollingAlgorithm;
}

// Returns the CScrollingAlgorithm for the focused monitor's active (or active special)
// workspace, or nullptr if that workspace isn't using the `scrolling` layout.
Layout::Tiled::CScrollingAlgorithm* resolveActiveScrollingAlgorithm();

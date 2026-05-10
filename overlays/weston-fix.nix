# Weston overlay to fix DRM format modifier assertion failure on AMD GPUs
# This fixes the crash: "Assertion `!weston_drm_format_has_modifier(format, modifier)' failed"
_final: prev: {
  weston = prev.weston.overrideAttrs (oldAttrs: {
    patches = (oldAttrs.patches or [ ]) ++ [ ./fix-duplicate-modifiers.patch ];
  });
}

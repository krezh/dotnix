# Weston overlay to fix DRM format modifier assertion failure on AMD GPUs
# This fixes the crash: "Assertion `!weston_drm_format_has_modifier(format, modifier)' failed"
final: prev: {
  weston = prev.weston.overrideAttrs (oldAttrs: {
    patches = (oldAttrs.patches or [ ]) ++ [
      # Patch to handle duplicate DRM modifiers gracefully
      (final.writeText "fix-duplicate-modifiers.patch" ''
        --- a/libweston/drm-formats.c
        +++ b/libweston/drm-formats.c
        @@ -413,7 +413,10 @@ weston_drm_format_add_modifier(struct weston_drm_format *format,
         	uint64_t *mod;

         	/* We should not try to add repeated modifiers to a set. */
        -	assert(!weston_drm_format_has_modifier(format, modifier));
        +	/* Skip duplicate modifiers instead of asserting - fixes AMD GPU crash */
        +	if (weston_drm_format_has_modifier(format, modifier)) {
        +		return 0;
        +	}

         	mod = wl_array_add(&format->modifiers, sizeof(*mod));
         	if (!mod) {
      '')
    ];
  });
}

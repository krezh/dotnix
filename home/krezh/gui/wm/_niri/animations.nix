{ ... }:
{
  programs.niri.settings.animations = {
    slowdown = 1.0;

    window-open = {
      kind.easing = {
        duration-ms = 150;
        curve = "ease-out-expo";
      };
    };

    window-close = {
      kind.easing = {
        duration-ms = 150;
        curve = "ease-out-quad";
      };
    };

    workspace-switch = {
      kind.spring = {
        damping-ratio = 1.0;
        stiffness = 1000;
        epsilon = 0.0001;
      };
    };

    horizontal-view-movement = {
      kind.spring = {
        damping-ratio = 1.0;
        stiffness = 800;
        epsilon = 0.0001;
      };
    };

    window-movement = {
      kind.spring = {
        damping-ratio = 1.0;
        stiffness = 800;
        epsilon = 0.0001;
      };
    };

    window-resize = {
      kind.spring = {
        damping-ratio = 1.0;
        stiffness = 800;
        epsilon = 0.0001;
      };
    };

    config-notification-open-close = {
      kind.spring = {
        damping-ratio = 0.6;
        stiffness = 1000;
        epsilon = 0.001;
      };
    };

    overview-open-close = {
      kind.spring = {
        damping-ratio = 1.0;
        stiffness = 800;
        epsilon = 0.0001;
      };
    };

    screenshot-ui-open = {
      kind.easing = {
        duration-ms = 200;
        curve = "ease-out-quad";
      };
    };
  };
}

_final: prev: {
  linux = prev.linux.overrideAttrs (old: {
    requiredSystemFeatures = (old.requiredSystemFeatures or [ ]) ++ [ "kernelbuild" ];
  });
}

{
  lib,
  stdenv,
  fetchFromGitHub,
  pkgs,
}:

stdenv.mkDerivation (finalAttrs: {
  pname = "low-latency-layer";
  # renovate: datasource=github-releases depName=Korthos-Software/low_latency_layer
  version = "0.2.0";

  src = fetchFromGitHub {
    owner = "Korthos-Software";
    repo = "low_latency_layer";
    tag = "v${finalAttrs.version}";
    hash = "sha256-mnGAH0m19wOkWEowpcPRHXQSc6HGYW+CFYxjPF2onk4=";
  };

  nativeBuildInputs = [ pkgs.cmake ];
  buildInputs = [
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-utility-libraries
  ];

  # vulkan-utility-libraries exports its cmake config as VulkanUtilityLibraries
  cmakeFlags = [ ];

  meta = {
    description = "Hardware-agnostic Vulkan layer for input latency reduction (Reflex/Anti-Lag)";
    homepage = "https://github.com/Korthos-Software/low_latency_layer";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    maintainers = [ ];
  };
})

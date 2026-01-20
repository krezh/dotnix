{
  flake.modules.nixos.amd = {pkgs, ...}: {
    hardware = {
      graphics = {
        enable = true;
        enable32Bit = true;
      };
      amdgpu = {
        opencl.enable = true;
        initrd.enable = true;
        overdrive.enable = true;
      };
    };

    environment = {
      sessionVariables = {
        WLR_BACKEND = "vulkan";
        PROTON_FSR4_UPGRADE = 1;
        AMD_VULKAN_ICD = "RADV";
        MESA_SHADER_CACHE_MAX_SIZE = "50G";
        __GL_SHADER_DISK_CACHE_SKIP_CLEANUP = 1;
      };
      systemPackages = with pkgs; [
        amdgpu_top
      ];
    };
  };
}

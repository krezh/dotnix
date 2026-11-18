//! Makes the images [`share`](crate::share) hands out, each a wgpu texture the frontend samples:
//! uploaded from pixels, or Vulkan images on island's own device, laid out in a DRM modifier the
//! compositor takes and exported to it as dma-buf.

use std::{
    os::fd::{FromRawFd, OwnedFd},
    sync::Arc,
};

use ash::{ext, khr, vk};
use tracing::warn;
use wgpu::hal::{api::Vulkan, vulkan::TextureMemory};

use crate::share::{self, ARGB8888, Image, Shared, XRGB8888};

/// Blue first in memory, as both DRM formats it takes are.
const FORMAT: vk::Format = vk::Format::B8G8R8A8_UNORM;
const HANDLE: vk::ExternalMemoryHandleTypeFlags = vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT;

pub struct Images {
    device: wgpu::Device,
    queue: wgpu::Queue,
    exporter: Option<Exporter>,
}

impl Images {
    pub fn offer(device: &wgpu::Device, queue: &wgpu::Queue) {
        let exporter = device
            .features()
            .contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_DMA_BUF)
            // Only reads what the device supports.
            .then(|| unsafe { Exporter::new(device) })
            .flatten();
        share::offer(Box::new(Self {
            device: device.clone(),
            queue: queue.clone(),
            exporter,
        }));
    }
}

impl share::Maker for Images {
    fn make(&self, size: (u32, u32), fourcc: u32, modifiers: &[u64]) -> Option<Shared> {
        if ![ARGB8888, XRGB8888].contains(&fourcc) {
            return None;
        }
        match unsafe { self.exporter.as_ref()?.export(size, modifiers) } {
            Ok(shared) => Some(shared),
            Err(error) => {
                warn!("cannot share an image with the compositor: {error}");
                None
            }
        }
    }

    fn upload(&self, (width, height): (u32, u32), pixels: &[u8]) -> Option<Image> {
        if width == 0 || height == 0 || pixels.len() != (width * height * 4) as usize {
            return None;
        }
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("picture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            size,
        );
        Some(Arc::new(texture))
    }
}

struct Exporter {
    device: wgpu::Device,
    /// The layouts the device samples an image of [`FORMAT`] in, as one plane.
    layouts: Vec<u64>,
    memory: vk::PhysicalDeviceMemoryProperties,
    fds: khr::external_memory_fd::Device,
    modifiers: ext::image_drm_format_modifier::Device,
}

impl Exporter {
    /// # Safety
    /// Uses the device's own Vulkan handles only to ask what it supports.
    unsafe fn new(device: &wgpu::Device) -> Option<Self> {
        let hal = unsafe { device.as_hal::<Vulkan>() }?;
        let raw = hal.raw_device();
        let instance = hal.shared_instance().raw_instance();
        let physical = hal.raw_physical_device();
        let mut list = vk::DrmFormatModifierPropertiesListEXT::default();
        let mut properties = vk::FormatProperties2::default().push_next(&mut list);
        unsafe { instance.get_physical_device_format_properties2(physical, FORMAT, &mut properties) };
        let count = list.drm_format_modifier_count as usize;
        let mut known = vec![vk::DrmFormatModifierPropertiesEXT::default(); count];
        let mut list =
            vk::DrmFormatModifierPropertiesListEXT::default().drm_format_modifier_properties(&mut known);
        let mut properties = vk::FormatProperties2::default().push_next(&mut list);
        unsafe { instance.get_physical_device_format_properties2(physical, FORMAT, &mut properties) };
        let sampled =
            vk::FormatFeatureFlags::SAMPLED_IMAGE | vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR;
        let layouts = known
            .iter()
            .filter(|known| {
                known.drm_format_modifier_plane_count == 1
                    && known.drm_format_modifier_tiling_features.contains(sampled)
            })
            .map(|known| known.drm_format_modifier)
            .collect();
        Some(Self {
            layouts,
            memory: unsafe { instance.get_physical_device_memory_properties(physical) },
            fds: khr::external_memory_fd::Device::new(instance, raw),
            modifiers: ext::image_drm_format_modifier::Device::new(instance, raw),
            device: device.clone(),
        })
    }

    /// # Safety
    /// Uses the device's own Vulkan handles; everything made is either handed to wgpu, which then
    /// owns it, or destroyed before returning.
    unsafe fn export(&self, (width, height): (u32, u32), modifiers: &[u64]) -> Result<Shared, String> {
        let hal = unsafe { self.device.as_hal::<Vulkan>() }.ok_or("the device is not Vulkan")?;
        let device = hal.raw_device();
        let usable: Vec<u64> = self
            .layouts
            .iter()
            .copied()
            .filter(|layout| modifiers.contains(layout))
            .collect();
        if usable.is_empty() {
            return Err("no layout both sides take".to_owned());
        }

        let mut chosen = vk::ImageDrmFormatModifierListCreateInfoEXT::default().drm_format_modifiers(&usable);
        let mut external = vk::ExternalMemoryImageCreateInfo::default().handle_types(HANDLE);
        let info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(FORMAT)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT)
            .usage(vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .push_next(&mut external)
            .push_next(&mut chosen);
        let image = unsafe { device.create_image(&info, None) }
            .map_err(|error| format!("cannot make the image: {error}"))?;
        let destroy_image = || unsafe { device.destroy_image(image, None) };

        let needs = unsafe { device.get_image_memory_requirements(image) };
        let types = &self.memory;
        let fits = |index: u32| needs.memory_type_bits & (1 << index) != 0;
        let Some(index) = (0..types.memory_type_count)
            .find(|&index| {
                fits(index)
                    && types.memory_types[index as usize]
                        .property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
            })
            .or_else(|| (0..types.memory_type_count).find(|&index| fits(index)))
        else {
            destroy_image();
            return Err("no memory the image fits in".to_owned());
        };
        let mut exported = vk::ExportMemoryAllocateInfo::default().handle_types(HANDLE);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let allocation = vk::MemoryAllocateInfo::default()
            .allocation_size(needs.size)
            .memory_type_index(index)
            .push_next(&mut exported)
            .push_next(&mut dedicated);
        let memory = match unsafe { device.allocate_memory(&allocation, None) } {
            Ok(memory) => memory,
            Err(error) => {
                destroy_image();
                return Err(format!("cannot give the image memory: {error}"));
            }
        };
        let undo = || unsafe {
            device.destroy_image(image, None);
            device.free_memory(memory, None);
        };
        if let Err(error) = unsafe { device.bind_image_memory(image, memory, 0) } {
            undo();
            return Err(format!("cannot give the image memory: {error}"));
        }
        let raw_fd = match unsafe {
            self.fds.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(memory)
                    .handle_type(HANDLE),
            )
        } {
            Ok(fd) => fd,
            Err(error) => {
                undo();
                return Err(format!("cannot export the image: {error}"));
            }
        };
        // A new descriptor, which `OwnedFd` then owns.
        let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };
        let mut laid = vk::ImageDrmFormatModifierPropertiesEXT::default();
        if let Err(error) =
            unsafe { self.modifiers.get_image_drm_format_modifier_properties(image, &mut laid) }
        {
            undo();
            return Err(format!("cannot read the image's layout: {error}"));
        }
        let plane = unsafe {
            device.get_image_subresource_layout(
                image,
                vk::ImageSubresource {
                    aspect_mask: vk::ImageAspectFlags::MEMORY_PLANE_0_EXT,
                    mip_level: 0,
                    array_layer: 0,
                },
            )
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let hal_texture = unsafe {
            hal.texture_from_raw(
                image,
                &wgpu::hal::TextureDescriptor {
                    label: Some("window"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    usage: wgpu::wgt::TextureUses::RESOURCE,
                    memory_flags: wgpu::hal::MemoryFlags::empty(),
                    view_formats: Vec::new(),
                },
                None,
                TextureMemory::Dedicated(memory),
            )
        };
        drop(hal);
        // The image is made from this device to the descriptor; the compositor fills it before it
        // is first drawn.
        let texture = unsafe {
            self.device.create_texture_from_hal::<Vulkan>(
                hal_texture,
                &wgpu::TextureDescriptor {
                    label: Some("window"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Bgra8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::wgt::TextureUses::UNINITIALIZED,
            )
        };
        Ok(Shared {
            fd,
            stride: plane.row_pitch as u32,
            offset: plane.offset as u32,
            modifier: laid.drm_format_modifier,
            size: (width, height),
            image: Arc::new(texture),
        })
    }
}

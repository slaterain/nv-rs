//! The device features nv-rs's additions to the Vulkan backend use (`vendor/README.md`): queried,
//! decided and enabled beside the published ones, with one call from each place in `adapter.rs`
//! that handles those.

// Our code in a vendored crate: no CI step lints it (`vendor/` is out of the workspace), and as a
// path dependency its warnings are not capped, so a warning here fails every build.
#![deny(warnings)]

use core::ffi::CStr;

use ash::{ext, khr, vk};

/// What a physical device offers our additions ([`Features::query`]), or what is enabled on a
/// device made from it ([`Features::enabled`]); a field of [`super::PhysicalDeviceFeatures`].
#[derive(Debug, Default)]
pub struct Features {
    /// What the queried device can do, or what the enabled set is for.
    caps: Caps,
    /// `VK_KHR_dynamic_rendering`, core in Vulkan 1.3 ([`super::dynamic_rendering`]); only an
    /// enabled set's is chained to the device's create info.
    dynamic_rendering: vk::PhysicalDeviceDynamicRenderingFeatures<'static>,
    /// `VK_EXT_graphics_pipeline_library` ([`super::pipeline_library`]); the same.
    graphics_pipeline_library: vk::PhysicalDeviceGraphicsPipelineLibraryFeaturesEXT<'static>,
}

impl Features {
    /// What the physical device offers, by its own `vkGetPhysicalDeviceFeatures2` and
    /// `vkGetPhysicalDeviceProperties2` (`None`: the instance has neither), for a device whose
    /// functionality the instance lets us use up to `api_version`.
    pub(super) fn query(
        get_properties: Option<&khr::get_physical_device_properties2::Instance>,
        phd: vk::PhysicalDevice,
        api_version: u32,
        supports_extension: impl Fn(&CStr) -> bool,
    ) -> Self {
        let Some(get_properties) = get_properties else {
            return Self::default();
        };
        let vulkan_1_3 = api_version >= vk::API_VERSION_1_3;
        let library_extensions = supports_extension(khr::pipeline_library::NAME)
            && supports_extension(ext::graphics_pipeline_library::NAME);

        let mut dynamic_rendering = vk::PhysicalDeviceDynamicRenderingFeatures::default();
        let mut library = vk::PhysicalDeviceGraphicsPipelineLibraryFeaturesEXT::default();
        let mut features2 = vk::PhysicalDeviceFeatures2::default();
        if vulkan_1_3 {
            features2 = features2.push_next(&mut dynamic_rendering);
        }
        if library_extensions {
            features2 = features2.push_next(&mut library);
        }
        unsafe { get_properties.get_physical_device_features2(phd, &mut features2) };

        let mut library_properties =
            vk::PhysicalDeviceGraphicsPipelineLibraryPropertiesEXT::default();
        if library_extensions {
            let mut properties2 =
                vk::PhysicalDeviceProperties2::default().push_next(&mut library_properties);
            unsafe { get_properties.get_physical_device_properties2(phd, &mut properties2) };
        }

        let dynamic_rendering = vulkan_1_3
            && dynamic_rendering.dynamic_rendering == vk::TRUE
            && !switched_off("NV_WGPU_DYNAMIC_RENDERING");
        let pipeline_library = dynamic_rendering
            && library_extensions
            && library.graphics_pipeline_library == vk::TRUE
            && !switched_off("NV_WGPU_LIBRARIES");
        Self {
            caps: Caps {
                dynamic_rendering,
                pipeline_library,
                fast_linking: pipeline_library
                    && library_properties.graphics_pipeline_library_fast_linking == vk::TRUE,
            },
            ..Self::default()
        }
    }

    /// What our additions use on a device with these (queried) features.
    pub(super) fn caps(&self) -> Caps {
        self.caps
    }

    /// The features to enable on a device for `caps`.
    pub(super) fn enabled(caps: &Caps) -> Self {
        Self {
            caps: *caps,
            dynamic_rendering: vk::PhysicalDeviceDynamicRenderingFeatures::default()
                .dynamic_rendering(caps.dynamic_rendering),
            graphics_pipeline_library:
                vk::PhysicalDeviceGraphicsPipelineLibraryFeaturesEXT::default()
                    .graphics_pipeline_library(caps.pipeline_library),
        }
    }

    /// Adds the enabled features to the device's create info.
    pub(super) fn add_to_device_create<'a>(
        &'a mut self,
        mut info: vk::DeviceCreateInfo<'a>,
    ) -> vk::DeviceCreateInfo<'a> {
        if self.caps.dynamic_rendering {
            info = info.push_next(&mut self.dynamic_rendering);
        }
        if self.caps.pipeline_library {
            info = info.push_next(&mut self.graphics_pipeline_library);
        }
        info
    }
}

/// A switch set to `off` (or `0`): the published path where ours would run, for checking it (devices
/// without our extensions take it) or getting round a driver's bug. `NV_WGPU_DYNAMIC_RENDERING`: render
/// pass objects on a Vulkan 1.3 device (and so no libraries); `NV_WGPU_LIBRARIES`: render pipelines made
/// whole.
fn switched_off(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "off" | "0"))
}

/// What our additions use on a device: a field of the backend's private capabilities.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Caps {
    /// Passes begun with `vkCmdBeginRendering`, pipelines made for attachment formats
    /// ([`super::dynamic_rendering`]).
    pub dynamic_rendering: bool,
    /// Render pipelines linked from pipeline libraries ([`super::pipeline_library`]); needs
    /// dynamic rendering.
    pub pipeline_library: bool,
    /// The driver links libraries quickly (`graphicsPipelineLibraryFastLinking`): reported only.
    pub fast_linking: bool,
}

impl Caps {
    /// The device extensions to enable for these.
    pub(super) fn extensions(&self) -> impl Iterator<Item = &'static CStr> {
        let library = self.pipeline_library;
        [
            khr::pipeline_library::NAME,
            ext::graphics_pipeline_library::NAME,
        ]
        .into_iter()
        .filter(move |_| library)
    }
}

//! The device features nv-rs's additions to the Vulkan backend use (`vendor/README.md`): queried,
//! decided and enabled beside the published ones, with one call from each place in `adapter.rs`
//! that handles those.

use ash::vk;

/// Features of a physical device, as queried for it ([`Features::query`]) or as enabled on a
/// device ([`Features::enabled`]); a field of [`super::PhysicalDeviceFeatures`].
#[derive(Debug, Default)]
pub struct Features {
    /// `VK_KHR_dynamic_rendering`, core in Vulkan 1.3 ([`super::dynamic_rendering`]).
    dynamic_rendering: Option<vk::PhysicalDeviceDynamicRenderingFeatures<'static>>,
}

impl Features {
    /// Adds the structures to fill to `features2` (`vkGetPhysicalDeviceFeatures2`), for a device
    /// whose functionality the instance lets us use up to `api_version`.
    pub(super) fn query<'a>(
        &'a mut self,
        api_version: u32,
        mut features2: vk::PhysicalDeviceFeatures2<'a>,
    ) -> vk::PhysicalDeviceFeatures2<'a> {
        if api_version >= vk::API_VERSION_1_3 {
            let next = self
                .dynamic_rendering
                .insert(vk::PhysicalDeviceDynamicRenderingFeatures::default());
            features2 = features2.push_next(next);
        }
        features2
    }

    /// What our additions use on a device with these features (queried ones).
    pub(super) fn caps(&self) -> Caps {
        Caps {
            dynamic_rendering: self
                .dynamic_rendering
                .is_some_and(|f| f.dynamic_rendering == vk::TRUE),
        }
    }

    /// The features to enable on a device for `caps`.
    pub(super) fn enabled(caps: &Caps) -> Self {
        Self {
            dynamic_rendering: caps.dynamic_rendering.then(|| {
                vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true)
            }),
        }
    }

    /// Adds the enabled features to the device's create info.
    pub(super) fn add_to_device_create<'a>(
        &'a mut self,
        mut info: vk::DeviceCreateInfo<'a>,
    ) -> vk::DeviceCreateInfo<'a> {
        if let Some(ref mut feature) = self.dynamic_rendering {
            info = info.push_next(feature);
        }
        info
    }
}

/// What our additions use on a device: a field of the backend's private capabilities.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Caps {
    /// Passes begun with `vkCmdBeginRendering`, pipelines made for attachment formats
    /// ([`super::dynamic_rendering`]).
    pub dynamic_rendering: bool,
}

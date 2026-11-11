//! Dynamic rendering: render pass instances begun with `vkCmdBeginRendering` (core in Vulkan 1.3)
//! instead of render pass and framebuffer objects. Added for nv-rs (`vendor/README.md`).
//!
//! A pass is begun from its attachments, and a pipeline is made for the attachments' formats
//! ([`vk::PipelineRenderingCreateInfo`]) rather than for a compatible `VkRenderPass`. Nothing is
//! kept per pass: no render pass object per set of attachment formats and operations, and no
//! framebuffer per set of views (the render pass path makes its framebuffers again after every
//! `reset_all`). Pipeline libraries need it: libraries made for formats link with each other,
//! where libraries made for render passes link only with ones made for compatible passes.
//!
//! What a render pass did on its own is the same here. wgpu-hal's render passes keep each
//! attachment in one layout from start to end (initial, subpass and final layouts are equal), so
//! they make no layout transitions and have no implicit external dependencies: the barriers
//! wgpu-core records between passes are all the synchronisation, in both paths. Load and store
//! operations, clear values, multisample resolves (averaged: wgpu resolves only formats whose
//! render pass resolve averages) and the view mask are carried over one for one.
//!
//! Used on Vulkan 1.3 devices with the `dynamicRendering` feature (every 1.3 device has it,
//! [`super::extras`]); older devices keep render passes.

use arrayvec::ArrayVec;
use ash::vk;
use core::num::NonZeroU32;

use super::conv;
use crate::CommandEncoder as _;

impl super::CommandEncoder {
    /// [`crate::CommandEncoder::begin_render_pass`] without render pass objects: the same pass,
    /// begun from its attachments, with the same debug marker, timestamps, viewport and scissor.
    pub(super) unsafe fn begin_rendering(
        &mut self,
        desc: &crate::RenderPassDescriptor<super::QuerySet, super::TextureView>,
    ) -> Result<(), crate::DeviceError> {
        let mut colors = ArrayVec::<
            vk::RenderingAttachmentInfo<'static>,
            { crate::MAX_COLOR_ATTACHMENTS },
        >::new();
        for cat in desc.color_attachments {
            let Some(cat) = cat.as_ref() else {
                // An unused slot has no view, as `VK_ATTACHMENT_UNUSED` in a render pass.
                colors.push(vk::RenderingAttachmentInfo::default());
                continue;
            };
            let view = if cat.target.view.dimension == wgt::TextureViewDimension::D3 {
                let key = super::TempTextureViewKey {
                    texture: cat.target.view.raw_texture,
                    texture_identity: cat.target.view.texture_identity,
                    format: cat.target.view.raw_format,
                    mip_level: cat.target.view.base_mip_level,
                    depth_slice: cat.depth_slice.unwrap(),
                };
                self.make_temp_texture_view(key)?.raw
            } else {
                cat.target.view.raw
            };
            let (load_op, store_op) = conv::map_attachment_ops(cat.ops);
            let mut info = vk::RenderingAttachmentInfo::default()
                .image_view(view)
                .image_layout(conv::derive_image_layout(
                    cat.target.usage,
                    cat.target.view.format,
                ))
                .load_op(load_op)
                .store_op(store_op)
                .clear_value(vk::ClearValue {
                    color: unsafe { cat.make_vk_clear_color() },
                });
            if let Some(ref at) = cat.resolve_target {
                info = info
                    .resolve_mode(vk::ResolveModeFlags::AVERAGE)
                    .resolve_image_view(at.view.raw)
                    .resolve_image_layout(conv::derive_image_layout(at.usage, at.view.format));
            }
            colors.push(info);
        }

        // A depth-stencil format's aspects each get an attachment of the one view, as the two
        // halves of a render pass's depth-stencil attachment (depth and stencil operations).
        let mut depth = None;
        let mut stencil = None;
        if let Some(ref ds) = desc.depth_stencil_attachment {
            let format = ds.target.view.format;
            let base = vk::RenderingAttachmentInfo::default()
                .image_view(ds.target.view.raw)
                .image_layout(conv::derive_image_layout(ds.target.usage, format))
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: ds.clear_value.0,
                        stencil: ds.clear_value.1,
                    },
                });
            if format.has_depth_aspect() {
                let (load_op, store_op) = conv::map_attachment_ops(ds.depth_ops);
                depth = Some(base.load_op(load_op).store_op(store_op));
            }
            if format.has_stencil_aspect() {
                let (load_op, store_op) = conv::map_attachment_ops(ds.stencil_ops);
                stencil = Some(base.load_op(load_op).store_op(store_op));
            }
        }

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: desc.extent.width,
                height: desc.extent.height,
            },
        };
        let vk_viewports = [vk::Viewport {
            x: 0.0,
            y: desc.extent.height as f32,
            width: desc.extent.width as f32,
            height: -(desc.extent.height as f32),
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let mut vk_info = vk::RenderingInfo::default()
            .render_area(render_area)
            .layer_count(desc.extent.depth_or_array_layers)
            .view_mask(desc.multiview_mask.map_or(0, NonZeroU32::get))
            .color_attachments(&colors);
        if let Some(ref depth) = depth {
            vk_info = vk_info.depth_attachment(depth);
        }
        if let Some(ref stencil) = stencil {
            vk_info = vk_info.stencil_attachment(stencil);
        }

        if let Some(label) = desc.label {
            unsafe { self.begin_debug_marker(label) };
            self.rpass_debug_marker_active = true;
        }

        // Start timestamp if any (before all other commands but after debug marker)
        if let Some(timestamp_writes) = desc.timestamp_writes.as_ref() {
            if let Some(index) = timestamp_writes.beginning_of_pass_write_index {
                unsafe {
                    self.write_timestamp(timestamp_writes.query_set, index);
                }
            }
            self.end_of_pass_timer_query = timestamp_writes
                .end_of_pass_write_index
                .map(|index| (timestamp_writes.query_set.raw, index));
        }

        unsafe {
            self.device
                .raw
                .cmd_set_viewport(self.active, 0, &vk_viewports);
            self.device
                .raw
                .cmd_set_scissor(self.active, 0, &[render_area]);
            self.device.raw.cmd_begin_rendering(self.active, &vk_info);
        };

        self.bind_point = vk::PipelineBindPoint::GRAPHICS;

        Ok(())
    }
}

/// The attachment formats a pipeline is made for, in place of a compatible render pass
/// (`VkPipelineRenderingCreateInfo`). The same rule as [`super::CommandEncoder::begin_rendering`]
/// for a depth-stencil format: depth for its depth aspect, stencil for its stencil aspect.
pub(super) struct RenderingFormats {
    colors: ArrayVec<vk::Format, { crate::MAX_COLOR_ATTACHMENTS }>,
    depth: vk::Format,
    stencil: vk::Format,
    view_mask: u32,
}

impl RenderingFormats {
    /// The formats of a pipeline's targets, or `None` on a device that uses render passes.
    pub(super) fn of_pipeline(
        device: &super::DeviceShared,
        color_targets: &[Option<wgt::ColorTargetState>],
        depth_stencil: Option<&wgt::DepthStencilState>,
        multiview_mask: Option<NonZeroU32>,
    ) -> Option<Self> {
        let caps = &device.private_caps;
        if !caps.extras.dynamic_rendering {
            return None;
        }
        let colors = color_targets
            .iter()
            .map(|target| {
                target
                    .as_ref()
                    .map_or(vk::Format::UNDEFINED, |t| caps.map_texture_format(t.format))
            })
            .collect();
        let (mut depth, mut stencil) = (vk::Format::UNDEFINED, vk::Format::UNDEFINED);
        if let Some(ds) = depth_stencil {
            let format = caps.map_texture_format(ds.format);
            if ds.format.has_depth_aspect() {
                depth = format;
            }
            if ds.format.has_stencil_aspect() {
                stencil = format;
            }
        }
        Some(Self {
            colors,
            depth,
            stencil,
            view_mask: multiview_mask.map_or(0, NonZeroU32::get),
        })
    }

    /// The structure to chain to the pipeline's create info.
    pub(super) fn info(&self) -> vk::PipelineRenderingCreateInfo<'_> {
        vk::PipelineRenderingCreateInfo::default()
            .view_mask(self.view_mask)
            .color_attachment_formats(&self.colors)
            .depth_attachment_format(self.depth)
            .stencil_attachment_format(self.stencil)
    }
}

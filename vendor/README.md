# Vendored crates

Third-party crates kept in the repository so that we can change them. The workspace's
`[patch.crates-io]` (root `Cargo.toml`) puts each in place of its crates.io copy; `exclude =
["vendor"]` keeps them out of the workspace, so `cargo fmt --all`, clippy and the workspace tests
leave their code as published.

## wgpu-hal 30.0.1

- Source: the crates.io package `wgpu-hal` 30.0.1 (gfx-rs developers, https://github.com/gfx-rs/wgpu,
  commit 40f4a34ebaf56f9a046231f54125ad046239d3f3, folder `wgpu-hal`), copied from the local cargo
  registry. Left out: the `examples` folder and the packaging files (`Cargo.toml.orig`, `Cargo.lock`,
  `.cargo_vcs_info.json`); git keeps `src/vulkan/descriptor.rs` with LF line ends like every `.rs`.
- Licence: MIT OR Apache-2.0, unchanged (`wgpu-hal/LICENSE.MIT`, `wgpu-hal/LICENSE.APACHE`).
- Why: the Vulkan backend is to make pipelines the way DXVK does (pipeline libraries, fast linking,
  dynamic state, optimised pipelines made in the background), so that a pipeline the game needs for
  the first time no longer stops a frame. wgpu and wgpu-core stay the published 30.0.1.
- Our changes are kept in their own files where possible, with few calls into the published code,
  so that they can be offered upstream (gfx-rs/wgpu #7302) and re-applied on a newer wgpu-hal.
  Every published file we changed says so in its first line.

### Changes

| Change | Files |
| --- | --- |
| Direct3D 12: compiled shaders kept as owned bytes, not as the compiler's blobs, whose reference count is not thread-safe (a use-after-free when several threads make pipelines from one shader with FXC, gfx-rs/wgpu #10547). Backport of upstream #10805, merged 2026-11-11 after 30.0.1 was published. | `src/dx12/mod.rs`, `src/dx12/device.rs`, `src/dx12/shader_compilation.rs` |
| Vulkan: dynamic rendering (`vkCmdBeginRendering`, Vulkan 1.3) in place of render pass and framebuffer objects on Vulkan 1.3 devices; older devices keep render passes. Pipelines are made for their attachment formats (`VkPipelineRenderingCreateInfo`) instead of a compatible render pass, which pipeline libraries need. | new `src/vulkan/dynamic_rendering.rs` and `src/vulkan/extras.rs` (the features our changes enable; `NV_WGPU_DYNAMIC_RENDERING=off` keeps render passes); calls from `src/vulkan/mod.rs`, `adapter.rs`, `command.rs`, `device.rs`; in `command.rs`, `begin_render_pass`'s colour-view and pass-start code moved unchanged into two helpers both paths call |

### Updating to a newer wgpu-hal

Copy the new version's package from the cargo registry over `wgpu-hal` (without `examples`),
re-apply the changes above (the new files as they are, the calls by hand), drop any change upstream
has taken, and move the pins in `crates/d3d9-wgpu/Cargo.toml` to the same version.

Some of our code mirrors published code and must follow it: diff these published functions
between 30.0.1 and the new version, and carry every change into ours.

- `src/vulkan/dynamic_rendering.rs` (`begin_rendering`) mirrors `command.rs`'s `begin_render_pass` and
  `end_render_pass` and `device.rs`'s `make_render_pass` (each attachment's load and store operations
  and layouts). The parts both use are shared helpers in `command.rs` (`color_attachment_view`,
  `pass_prologue`) and `conv.rs` (`make_attachment_key`, `map_attachment_ops`).
- `RenderingFormats` in the same file mirrors the render pass key `create_render_pipeline` makes.

After re-vendoring, replay a recording on Vulkan both ways (`NV_WGPU_DYNAMIC_RENDERING=off` takes the
published render pass path) and compare the pictures with the old build's.

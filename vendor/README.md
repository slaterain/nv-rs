# Vendored crates

Third-party crates kept in the repository so that we can change them. The workspace's
`[patch.crates-io]` (root `Cargo.toml`) puts each in place of its crates.io copy; `exclude =
["vendor"]` keeps them out of the workspace, so `cargo fmt --all`, clippy and the workspace tests
leave their code as published.

## wgpu-hal 30.0.1

- Source: the crates.io package `wgpu-hal` 30.0.1 (gfx-rs developers, https://github.com/gfx-rs/wgpu,
  commit 40f4a34ebaf56f9a046231f54125ad046239d3f3, folder `wgpu-hal`), copied from the local cargo
  registry. The `examples` folder was left out; nothing else.
- Licence: MIT OR Apache-2.0, unchanged (`wgpu-hal/LICENSE.MIT`, `wgpu-hal/LICENSE.APACHE`).
- Why: the Vulkan backend is to make pipelines the way DXVK does (pipeline libraries, fast linking,
  dynamic state, optimised pipelines made in the background), so that a pipeline the game needs for
  the first time no longer stops a frame. wgpu and wgpu-core stay the published 30.0.1.
- Our changes are kept in their own files where possible, with few calls into the published code,
  so that they can be offered upstream (gfx-rs/wgpu #7302) and re-applied on a newer wgpu-hal.
  Every published file we changed says so in its first line.

### Changes

None yet: this is the published code.

### Updating to a newer wgpu-hal

Copy the new version's package from the cargo registry over `wgpu-hal` (without `examples`),
re-apply the changes above (the new files as they are, the calls by hand), drop any change upstream
has taken, and move the pins in `crates/d3d9-wgpu/Cargo.toml` to the same version.

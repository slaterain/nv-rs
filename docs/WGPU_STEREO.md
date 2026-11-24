# `NV_3D` playtesting

`NV_3D=anaglyph` uses red-left/cyan-right glasses. `NV_3D=crosseyed` puts the right
view on the left, preserving each view's aspect ratio. Unset the variable (or set
it to `off`) for the usual image. Both work on the wgpu window; real Direct3D 9
would need a separate compositor and is unchanged.

This is a port-added screen-space mode, not a second camera or a VR path. At the
existing world-drawn seam (after `00873200` in `00870bd0` / `00870a00`,
FalloutNV.exe 1.4.0.525), wgpu saves disparity from the live perspective frustum
and the world depth before the first-person arms clear it. Present then samples
the finished image twice at depth-dependent offsets and applies the brightness
ramp. Frames without that depth stay flat (duplicated for cross-eyed viewing).
Back-buffer captures are untouched; the `wgpu-window-*.bmp` image includes stereo.
Geometry hidden from the original camera, HUD, transparent surfaces, water drawn
later and first-person arms can show artifacts. The separation and convergence
are conservative display defaults, not original game settings.

The Linux Vulkan GPU regression passed for near/far disparity, eye order,
multisampled depth, the depth clear before the arms, a frame without fresh depth,
and the gamma ramp. The shader also validates without a graphics adapter.
`cargo check --tests` passed for the three changed crates on the Windows and
macOS targets. The Linux play build reached the main menu at 1280×720 with 4×
MSAA; the automated input route did not load the save, so gameplay is not yet
verified. The Linux `NV_D3D9=real` run logged that stereo was ignored before
ending because native Direct3D 9 is unavailable on Linux.

Next: inspect near edges, the HUD and the arms through red/cyan glasses in first-
and third-person play. Launch without `NV_MOVE_KEYS` and `NV_MOVE_MOUSE` to keep
live mouse input. The workspace test and Clippy runs had not finished at handoff.

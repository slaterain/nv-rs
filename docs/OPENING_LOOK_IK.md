# Opening Doc look IK trace

## Follow-up output trace, 2026-10-04

No runtime gaze change is implemented. Ghidra assembly confirms mode1's
active call at `00c7aa8f..00c7aab0`, selected node+0x144, second transform
selector+0x146 and quaternion cache+0x170..0x17c. `00c78610` rebuilds the
selected/subtree transforms and passes its selected quaternion through
`00c755e0`, which caps the relative rotation and writes both node and cache.

The final `005611c0`/`005611e0` correction is now resolved as quaternion
normalization: sum four squared components, reciprocal square root with one
Newton refinement, then scale all four components. The native implementation
uses SSE approximations, so bit-identical results need separate validation.

The cap selects `01267d24` or `01267d30` using controller+0xb2 and converts
degrees to radians. No delta-time factor appears in this helper: the cap is
per invocation. Although both globals are zero in the executable image,
constructors `00fbd100`/`00fbd130` register `fAngleMax:LookIK` default 3.5
and `fAngleMaxEase:LookIK` default 1.0 respectively. Post-INI runtime values
remain unverified; see the constructor evidence below.
The second mode0 call uses a temporary transform and changes+0x181 and
feedback metrics; it is not merely an inert cache update.

Remaining requirements: verify post-INI cap values and skeleton transform
spaces, establish the
quaternion product convention, then compare a port in the original game.
Private evidence: `nv-re/work/m1-overnight-2026-10-04/gaze/FINDINGS.md` and
`hashes.sha256.json` (read-only Ghidra disassembly/helper exports). Analysis
image: FNV1.4.0.525, unpacked SHA256
`19406942E48724D797300C4EA6BE9AC69A32F670B8C35DB09279A2258422739F`.

## Earlier trace

Research status, 2026-10-03. FNV executable addresses below refer to the
existing `nv-re/decomp/codex-m1` Ghidra project. No runtime behavior has been
implemented from this trace.

## Confirmed native path

Doc's actor update `008a3100` chooses a tracked actor, obtains its anchor from
the target actor's virtual `+0x194` method, and calls `008a3b70` on the actor's
controller stored at actor `+0xac`. `008a3b70` writes the supplied xyz into
controller `+0xd0` (fourth component zero), then invokes `00c75580` to enable
the look system. The controller is created by `0087e130` through constructor
`00c7f060`; its initialization path includes a specific LookIK initialization
failure diagnostic.

The visual consumer is verified, including register/stack roles:
`00c7d630` has the controller in ECX and passes `this+0x10` on the stack to
`00c7aa60` (`dis_00c7d630.txt`, `00c7d744-00c7d747`). The callee keeps ECX as
the controller and calls `00c78160` with that same `this`. `00c78160` reads
controller `+0xd0`, alongside its matrix at `+0x10`, and calls `00c7f840`.
That helper subtracts matrix translation from target xyz, applies the inverse
controller rotation to get a local direction, and normalizes it. Thus the
target-to-local-direction transform is established and this is the actor's
LookIK path, not a generic look-at inference.

`00c78160` records whether the local target is below in controller `+0x1a4`.
The below-target branch is conditional on state byte `+0xb2`; it uses signed
clamp values `+5` and `-5` before mapping through selected skeleton
transforms. It also limits movement of the smoothed point to 5 native units
per update (`DAT_011b05a8`, `00c78598-00c785f0`). The precise coordinate
meaning and units of those clamps are not yet established.

`00c7aa60` calls `00c78610` with `param_2=1` for the active result, then
passes a second `param_2=0` call using a temporary matrix. `00c78610` reads the
same target at controller `+0xd0`, selected skeleton index `+0x144`, and
per-mode index at `+0xf4`/`+0xf6`. Its body builds and composes quaternion
transforms and writes node transforms, but the Ghidra output is too large and
ambiguous to claim the final rotation formula or limits without tracing the
exact output writes and helpers.

## Unresolved, exact next trace

The controller constructor initializes short `+0x144` to `-1`. The setter is
now found in `00c79340`: it writes the result of `00cdd390` to
`controller + mode * 0x50 + 0xf4`; mode1 therefore writes `+0x144`.
`00c7de60` calls it with mode1. Its sole caller is `0087e575`, passing the
string returned through `0063d040 -> 00559450` from the body-part iterator
at local `-0x290`. **Assembly confirms the name is BPNI**: `0063d04a`
adds0x14 before the string getter; Ghidra's C output omitted that adjustment.
The loader `005e427c` checks BPNI and `005e42b0` writes its string at+0x14.
The iterator (`005e5320`) scans 15 BPTD part slots+0x34, selecting flag0x02;
`0087e940` selects flag0x20 for LookIK. DefaultBodyPartData0000001D's Head
entry has flags0x3b and BPNI `Bip01 Head`. Thus mode1's selected node is
data-driven, and is Bip01 Head for the standard human body data. BPNN is
different (Bip01 Neck1, stored at+4); do not confuse the two. The temporary
Ghidra project lock cleared. Evidence: look-node-getter.txt,
look-body-strings.txt and look-body-data.txt in nv-re/work/codex-m1.

For output rotation, continue `00c78610` from its `param_2=1` call in
`00c7aa60`: label each transform source (`+0x110..+0x11c`, per-node data,
target local direction), follow the final writes to the selected node's
rotation, and decompile only helpers on that dataflow. Separately trace
`00c755e0` if it contributes the angle or easing value. Confirm whether the
second (`param_2=0`) call is just a temporary/cache update. These are the
minimum gaps before a runtime implementation can reproduce native behavior.

The opening's `SayTo Player` is a scripted talk event, not a dialogue package;
it is an explicit reason for Doc to have the player as a target, but the
viewer currently has no native actor look-target/effect wiring. Random
`SitChairRelax` idle variation is separate from the absent gaze behavior.
`viewer/src/faces.rs` documents that head-turn and eye tracking are absent;
the seated body-turn guard in `viewer/src/ai.rs` must remain intact.

## Evidence locations

Follow-up, 2026-10-04: `00c755e0` caps a relative quaternion per invocation
(no delta time in this helper). Constructors `00fbd100` and `00fbd130`
register `fAngleMax:LookIK` default 3.5 at `01267d24` and
`fAngleMaxEase:LookIK` default 1.0 at `01267d30`. Controller byte `+0xb2`
selects the value; multiplication by 0.0174533 converts degrees to radians.
These are constructor defaults, not verified post-INI runtime values.
`005611c0`/`005611e0` normalize the quaternion using four-component norm
and reciprocal square root refinement, with a zero guard. The mode0 call
updates `+0x181` and feedback metrics; it is not merely an inert cache.
Mode1 selects the `+0x144` node and caches quaternion `+0x170..+0x17c`.
The setup is resolved below; transform conventions and original-game
comparison still block a faithful runtime port. Private evidence and the
hash manifest: nv-re/work/m1-overnight-2026-10-04/gaze/. Unpacked executable
1.4.0.525 SHA256
`19406942E48724D797300C4EA6BE9AC69A32F670B8C35DB09279A2258422739F`.

## Setup assembly follow-up, 2026-10-04

`00c7de60:00c7dea8` explicitly passes mode1 to `00c79340`; ECX holds the
controller and there are three stack arguments. Mode record offsets below
are relative to `controller + mode * 0x50`. `+0xf4` is the name-matched node
index; `+0xf6` is its parent/link index. The table builder `00c7e3b0` copies
that short from source-node+0x40; an ancestor walk in `00cea810` corroborates
its role using the sibling table with the same constructor and layout.

Setup copies a 13-float transform from `*(controller+0x58)+0x68` and calls
`0043f8d0` with the matrix at controller+0x60 as receiver. Its row-major
product is receiver times input. The semantic transform spaces remain
unresolved; the pointer dereference must not be flattened into an offset.

For the matched node and its parent, the fixed basis is `(0,1,0,0)`.
Given quaternion `(x,y,z,w)`, the written XYZ values are
`2*(x*y+z*w)`, `2*(y*y+w*w-0.5)`, `2*(y*z-x*w)`. A fourth lane is `2*y*w`.
The matched-node value at `+0x100` scales all four lanes using an approximate
reciprocal square root of the **XYZ** squared magnitude; the parent value
at `+0x110` is not normalized. These are derived direction values, not two
quaternions. The fourth lane's consumer meaning remains unresolved.

This is research only: no runtime port, generated regression, or original-
game comparison. Private assembly/decompiler exports and the verified
manifest are under `gaze/`; `SETUP_ASSEMBLY.md` records the exact calls and
constants. A mode1 override must also be preserved: when the object pointer
at `*(controller+0x2a4)+0x1c` is nonnull, its four-float values at+0x10 and
+0x20 replace the derived parent and matched-node values respectively.
`00c7de60` creates that outer 0x30-byte wrapper only after successful setup,
when controller+0x190 is enabled and the wrapper slot is null. It copies
mode1's just-derived parent/matched vectors into wrapper+0x10/+0x20. Later
setup reads those snapshots back. Wrapper+0 points to a separate 0x48-byte
table object; its arrays are not the source of these two vectors. This is
a runtime cache in the traced path; no authored data source was found.
Other writers and actor-facing semantic names remain unresolved.

The selected-node recompute path at `00c78c2a` calls `00c66320` with
`conjugate(parent) * selected`, in xyzw order. A parent link of -1 instead
copies the transform. Direct four-lane reads prove mask010c4c00 negates XYZ
but preserves W; the separate mask010c44d0 negates all four lanes. Inferring
the full mask from its first dword gives the wrong rotation. The product
equals inverse-parent multiplication only for a unit parent quaternion.
Mode-record+0x130 then optionally enables the previously traced limiter.
This recompute product alone is not the complete target-to-head solver.

`OUTPUT_COMPOSITION.md`, `OVERRIDE_SOURCE.md` and the now 92-entry private
manifest include the fresh assembly and lane reads. Next: resolve the
transform-buffer spaces and follow the target-direction contribution through
the full solve before implementing or claiming original-game fidelity.

Decompilation and disassembly: `%USERPROFILE%\\nv-re\\decomp\\codex-m1`
(`008a3100`, `008a3b70`, `00c7f060`, `00c7d630`, `00c7aa60`, `00c78160`,
`00c7f840`, `00c78610`). Opening route and package evidence:
`docs/OPENING.md` and `%USERPROFILE%\\nv-re\\work\\codex-m1\\animation-repro.log`.

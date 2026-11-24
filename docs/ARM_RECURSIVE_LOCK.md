# Recursive-lock publication on native host threads

## Verified mechanism

The game's recursive lock is acquired at `0040fbf0` and released at
`0040fba0`. In the owned GOG en-US 1.4.0.525(a), build 55068 executable,
`0040fbc7` clears its owner word with an ordinary x86 store. That processor's
store ordering publishes earlier protected writes before another thread can
claim the lock. Copying the instruction as a plain ARM store does not preserve
that ordering.

Both the translated and typed implementations now use a release store of the
aligned owner word when memory is shared by host threads. It pairs with the
acquire compare-exchange already used to claim the lock. The recursive
fast-path read is atomic as well. Nested releases retain the owner as before;
stepped execution keeps its ordinary memory accesses. The unaligned/unmapped
fallback retains the existing memory-access path with a release fence.

## Regression evidence

`crates/engine/tests/recursive_lock_publication.rs` uses no game files. Four
real engine threads make 100,000 updates each to 16 protected words and check
that they agree whenever the lock is held. It exercises both production lock
implementations, not stand-ins for the lock or interlocked operations.

On Apple M1 Max, before the change, the typed implementation failed this test:
a thread observed inconsistent protected words. The translated implementation
passed that run. Both passed after the change, including ten consecutive runs
(eight million protected updates total). Six existing `typed_dispatch` tests
matching `lock` also passed after the change.

A separate small native publication probe reproduced inconsistent words with
an acquire CAS and relaxed unlock store; with a release unlock it completed
six million updates without a mismatch. This supports the mechanism, while
the engine regression exercises the actual changed functions.

## Gameplay evidence and limits

Three observed stops motivated this investigation:

- `00a04200`: a zero-address virtual call during tile updating;
- `00a59d30`: property lookup reached `00a5e280` without its required argument;
- `00a9b570`: an unmapped read at `00000008` during alpha sorting.

A subsequent diagnostic run repeated the first stop at frame 20,573 and its
native backtrace identified `Tile::UpdateTile` as the virtual caller.
These symptoms are compatible with corrupted shared lists: the list-node pool
uses this lock. Their common cause has not been demonstrated by tracing the
first invalid pointer write, and the change is not yet proof that all three
stops are resolved. A good unmodified diagnostic run lasted over 13,000 frames
before it eventually stopped, so one short successful run is insufficient.

The corrected native play build has started with real threads, the render
thread, Metal, 3024x1898 rendering and 4x multisampling. Gameplay reproduction,
Windows/Linux runs, and the complete workspace suite remain outstanding.
Next: repeat the reported play routes with this build; inspect the captured
native backtrace if any stop recurs.

## Follow-up: the scrap-buffer pool's inlined unlock

The first corrected play run later stopped at frame 91,664 on game thread 4:
`006bbc50 < 00698f40 < 006c8170 < 00b023f0`, an unmapped read at `0000001b`
in `NavMeshObstacleCutter` destruction. The later main/render-thread stop was
propagation of that worker failure. This is a different signature from the
three earlier stops; its exact invalid pointer write has not been captured.

The cutter uses `ScrapHeapManager` buffers. Its private `unlock_buffers` copies
the compiler-inlined BSSpinLock release (for example the owner stores at
`00aa5bea` and `00aa5c68`), so it never reaches `0040fba0`. The first fix did
not cover these releases. The same release-store rule is now applied there.
The table operations, recursion count, and stepped-memory path stay unchanged.

`crates/engine/tests/scrap_heap_publication.rs` seeds eight synthetic buffers
and exercises the actual `RequestBuffer` (`00aa59b0`) and `ReleaseBuffer`
(`00aa5b70`) from four real engine threads, 100,000 times each. Atomic ownership
markers catch a buffer handed to a second thread while still in use; payloads
must stay consistent, and the final table must contain all eight buffers once.
No game files or UI are involved.

On M1 Max, with the general lock fix already present but the private unlock
unchanged, the test failed twice, specifically detecting duplicate buffer
ownership (not an unknown buffer id). With the private unlock corrected, it
passed, including ten repeat runs: four million handouts and four million
returns without duplicate ownership or inconsistent payloads. All 36 existing
scrap-heap unit tests and Clippy for engine-bs all targets also passed on that
combined local candidate.

This demonstrates a buffer-sharing race capable of corrupting cutter data;
it does not prove that it caused the captured `0000001b` fault. Gameplay was
not relaunched after the user requested that it stay closed. Retesting that
route remains the next validation step. Temporary cutter instrumentation is
excluded from the fix.

## Inline node-pool releases (2026-11-22)

The generic unlock is not the pool's only release path. The shared 12-byte
list-node pool (`011c5f58`, owner `011c5f80`, recursion count `011c5f84`) also
has inline final-owner clears in the renderer, scene objects and list helpers.
Those copies still used ordinary stores after the generic unlock was fixed.
For example, the owned 1.4.0.525 executable publishes the returned free head
at `00b65035` before clearing its owner at `00b6503d`; its second geometry-list
branch does the same at `00b65094` and `00b6509c`. `00b63e90` has nested node
returns and a final outer release.

The 33 inline final-owner stores for this pool now use
`Mem::set_u32_release`. This preserves the original recursion tests and write
order, supplies Release ordering with shared host memory, and keeps ordinary
tracked memory writes in stepped execution. The refresh covers five more clears of the same owner word in
BSBatchRenderer, BSShaderAccumulator and NiXenonRenderer.

A subsequent gameplay diagnostic captured a dirty UI queue node changing from
a live TileImage to a node containing a NiTriStrips geometry, without an
intervening UI pop. This establishes list-node corruption; it does not identify
the exact writer or prove missing publication was its only cause. Generic
allocator/free stress passed before this change and did not exercise these
inline renderer releases. Bounded four-thread stress of actual renderer spare-list returns
also passed before the change, including 400,000 returns each through typed and
translated paths with play-profile optimizations: this is a source-proven ordering correction,
not a reproduced red-to-green gameplay fix. Gameplay validation remains open.

The synthetic-data integration test `node_pool_publication.rs` covers four
production return paths: typed and translated `00b63e90` spare-chain releases,
translated `00b602d0` single-node removal, and typed `00b64f90` accumulator
cleanup. Each case uses four host threads and 400,000 allocation/return pairs.
It checks exclusive node ownership, payload integrity, publication of 16
protected words, the final free chain, and empty owner/count state. It excludes
refill and actual rendering; unrelated batch-end work is a no-op test double.

On Apple M1 Max the original 28-site patch passed all four cases (1.6
million allocation/return pairs), and both existing recursive-lock publication
tests passed. Four focused shader differential tests passed: `release_spare`,
`draw_sorted`, `accumulator_finish`, and `accumulator_draw`. The three memory
helper tests, affected-crate Clippy with warnings denied, explicit
`aarch64-apple-darwin` test compilation, and the combined local play build
passed. Its headless check against the local mod profile exited successfully.
The full workspace suite and Windows/Linux runs were not performed for this
batch; the full shader suite was interrupted in favor of the focused checks.
The game has not been reopened for gameplay validation.

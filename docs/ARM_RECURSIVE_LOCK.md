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

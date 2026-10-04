# Shared native research workflow

Start with the current M1 topic handoff and its executable addresses. Read
the matching Ghidra export, inspect associated game data, then compare in the
original game. Keep exports, tools, databases, recordings and manifests in
the private research tree outside this repository.

## gamedb evaluation, 2026-10-04

Evaluated [gamedb](https://github.com/smileybaal/gamedb) at commit
`7054201291d704d64cdf37a9d3573c5d16a81fcf`, after reading its README, CLI,
parser, indexer and tests. Its release suite passed 25 integration tests,
including the built-in self-test.

The four-export player look-lock probe indexed three functions and one call
edge, with zero reported skipped/failed files. It silently missed `005cc4f0`:
the Ghidra declaration separates its return type from the function-name line.
Consequently the graph recovered removal's call to `005cc7a0`, but omitted
the assignment caller. Name-only edges also cannot establish the player-only
branch guard. A broad read of the mouse handler returned 754 lines.

**Decision:** retain direct, targeted Ghidra queries for now. Full-corpus
indexing was not justified. The private export inventory has 5,210 candidate
files, 943 exact duplicate copies and 45 same-basename groups with differing
contents; executable/export provenance is insufficient to merge those variants.
File counts and hashes do not establish extraction or behavioral correctness.

Private evidence: `%USERPROFILE%/nv-re/work/gamedb-eval-2026-10-04/` contains
`FINDINGS.md`, `player-look-manifest.json`, `export-inventory.json`, the tool
snapshot, four copied exports and the fresh `player-look-gate.sqlite` database.
No existing database, Ghidra project or game file was replaced.

## Queries across sessions

1. Follow the topic's address and executable hash; use `rg` to locate that
   address or function in the matching private export directory. Read only
   the relevant function or range. Inspect assembly when calling conventions
   or omitted register adjustments make the C output ambiguous.
2. Record source path, address, executable identity, export hash and guarded
   branch conditions beside the finding. Link the Rust implementation and
   generated regression, and distinguish original-game observation from tests.
3. If revisiting gamedb, select a version-consistent, hashed source snapshot
   and a new named database. Start with `search`, then a narrow `graph`/`read`.
   Compare extracted functions against an expected address list and verify
   edges in Ghidra. Zero skipped files is not an extraction-completeness check.
4. Do not combine the conflicting exports or repair/index the entire corpus
   until a bounded probe demonstrates useful retrieval and correct extraction.
   The private findings contain the exact commands needed to repeat this probe.

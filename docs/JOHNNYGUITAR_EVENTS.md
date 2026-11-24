# Selected JohnnyGuitar commands and events

This slice extends the partial built-in JohnnyGuitar API with actor-value,
process-level, quest/perk, sleep/settings, selection and cell-seen/dying
callbacks, and the associated movement, form and perk-browser queries.
It follows JohnnyGuitar source revision `59049a0`, with source paths and
FalloutNV.exe 1.4.0.525 seam addresses next to each implementation. The existing
native command records, parameter readers and callback/lambda ownership are
retained. No complete JohnnyGuitar plugin/version is advertised.

The engine changes expose the original callback boundaries rather than invoking
callbacks for every call of an underlying utility. Actor-value reads preserve
the original getter test while removing only the source-patched branch. Perk
callback ranks narrow to a byte without narrowing the original native rank.
Cell-seen callbacks retain their original position before extra-data lookup;
OnDying is absent on the queued-kill early return. Tests exercise registration,
filters, deferred dispatch, callback lifetime, branch placement and raw arguments.

This branch builds on the native config/form APIs and JAM API foundation.
The tests establish the selected API behavior; they do not establish complete
compatibility with the mods that consume it. The user's combined candidate has
exercised water bottling, but original-game differential execution of all these
callbacks and Windows/Linux gameplay remain open.

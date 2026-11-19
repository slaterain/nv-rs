# Unix play-profile selector

`play.sh` uses `NV_RS_PROFILE` to select the `game` or `mods` save/settings
profile. The machine-table reader uses the same variable. `NV_PROFILE` is
reserved for the engine's sampling profiler.

Previously the launcher exported `NV_PROFILE=game` or `mods` on every start.
The engine treated that as a diagnostic run: player INI choices were ignored,
the fixes preset and real-thread defaults were lost, and no default renderer
was selected. This was observed during an Apple Silicon play-build start.

The launcher tests cover both save profiles and assert that selecting one
does not export `NV_PROFILE`. Profile paths and the explicit profiler switch
are unchanged. No game assets are needed for the regression tests.

Next validation: the Unix launcher and platform CI jobs on all supported hosts.

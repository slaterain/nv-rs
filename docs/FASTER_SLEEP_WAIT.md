# Faster Sleep Wait

With lStewieAl's Tweaks enabled, set `bFasterSleepWait=1` under `[Tweaks]`
in `Data\NVSE\Plugins\nvse_stewie_tweaks.ini`. The feature is off by
default. It uses the existing main and overlay INI precedence.

Two optional values under `[Sleep Wait]` control the comparison.

- `iWaitTimeMS` defaults to 300. The native reader accepts fractional
  milliseconds and applies the same floating-point conversion as Tweaks.
- `bDontScaleWithTimescale` defaults to 1. With it on, the comparison divides
  accumulated time by the game's speed. Set it to 0 to use accumulated time directly.

The Rust implementation follows lStewieAl's Tweaks 10.05. It preserves the
original menu's cancellation and one-hour-per-update limit. Tests cover the
comparison boundary, timescale, cancellation, defaults, and INI overlays.

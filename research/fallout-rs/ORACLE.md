# The oracle: calling FalloutNV.exe's own functions from a checker

This page describes how fallout-rs checks a Rust port of a game formula
against the original program's function, on thousands of random inputs,
without the game ever running. It is written so the method can be rebuilt
from the description; no code is included. In fallout-rs it is the
`fo-oracle` crate (about 12,000 lines including the checks; the process
and stub machinery is a small part of that).

Ground rule: the tool works on **a readable copy of FalloutNV.exe
1.4.0.525** kept in a private folder outside the game installation. It
never starts, opens for writing or reads from the installed game folder,
and it never runs the game.

## How it relates to nv-rs's `research/nv-oracle`

nv-rs's `nv-call` maps the program image into its own 32-bit process and
calls a function there; `nv-probe` hooks a running game. The fallout-rs
oracle sits between the two: the operating system's own loader maps the
program into **its own process**, so relocations, imports and static data
are exactly as in the game, but the program's main thread is never
allowed to start. The two approaches are complementary; the second is
useful when a function reaches into imported libraries or into the
program's global data, and needs no 32-bit host.

## The steps

1. **Create the process suspended inside a kill-on-close job.** Create a
   job object with the "kill on job close" limit, then create the program
   process with the suspended flag (working directory: the private
   folder), and assign it to the job before anything else. The main
   thread's handle is closed and the thread is never resumed. If the
   checker dies for any reason, closing the job kills the process.
2. **Check what is loaded.** Read the program's headers and code from the
   process's memory and compare the bytes of every target function with
   the file on disk (all identical in every run so far), and record a hash
   of the file in the report header.
3. **Drop unwanted imports.** Before any thread runs, rewrite the import
   directory in the mapped image so that DLLs whose start-up code must not
   run are skipped (fallout-rs keeps 14 of the 17). The other system DLLs
   stay, because many functions call into them.
4. **Let the loader finish, and nothing else.** The first remote thread
   created in a suspended process makes the loader do its per-process
   initialisation (the remaining DLLs' entry points). Point that thread
   at a tiny stub that returns a known value (fallout-rs uses 0x1234) and
   check the value. The program's own entry point and C runtime start-up
   never run: no static initialisers, no window, no game code.
5. **Allocate two blocks** in the target process: a code block
   (64 KB, read/write/execute) for call stubs and fake virtual functions,
   and a data block (512 KB) for fake objects, arguments and result
   records.
6. **Build a call stub per call**, by hand, as x86 machine code:
   load the x87 control word the checker chose (fallout-rs uses 0x027F:
   53-bit precision, round to nearest, all exceptions masked) and the SSE
   control word if needed; push the arguments right to left; load ECX
   (thiscall) or ECX and EDX (fastcall); call the target through a
   register (`mov eax, target; call eax`, so one stub shape fits every
   address); store EAX, EDX and ST(0) (as a double) into the result record
   in the data block; for cdecl pop the arguments; return from the thread
   procedure. Run it with a remote thread, wait for it, read the record.
   When many targets are called, one trampoline that reads the target
   from the data block avoids running out of stub space.
7. **Stand in fake objects.** Many formulas take an actor, a weapon or an
   "actor value owner" and read through virtual calls. Build a fake object
   in the data block whose vtable (also in the data block) points at small
   stubs in the code block: return a value from a table (`mov eax,
   [table + 4 x index]` style for integers, `fld dword [..]` for floats)
   and `ret n` with the right stack cleanup. Fill only the slots the
   function is seen to call; the disassembly says which. Where the
   function reads a global (the player pointer, the time globals, the
   hardcore need clocks, the actor-value info table), write a fake object's
   address or a value into the program's own data section, which is
   zero-filled because no initialiser ran.
8. **Settings.** New Vegas builds its Setting objects at run time, so in
   the suspended image every value word is zero (fallout-rs counted 4,581
   of 4,581). Write each setting's built-in default into its object before
   the checks (the value sits at object + 4). To get the defaults right,
   fallout-rs ran each registration function itself inside the process
   with the global "collection" pointers aimed at a fake collection and
   the C runtime's `atexit` patched to return (restored afterwards):
   4,459 of 4,459 names and defaults confirmed against its table. Checks
   that vary settings overwrite the object, call, and restore.
9. **Random inputs, then edge cases.** Per check: a seeded generator,
   1,000 to 1,600 samples, with ranges a little wider than the game uses
   (negative skills, Endurance up to 14, conditions -0.2 to 1.2), a share
   of samples placed exactly on limits (about one in four), hand-picked
   cases where float and double arithmetic part, and the function's
   settings randomised in about three samples of ten.
10. **Compare and explain.** Integers must match exactly; floats within
    1e-6 relative unless the check says why not. When the Rust differs,
    write the reading of the program you believe in as a second small
    function and run it on the same inputs; the report then says "proposed
    fix: N of N match". A mismatch is only closed when such a reading
    matches every sample.
11. **Random-number consumers.** For a function that draws from the
    program's generator, read the generator state from the process before
    the call, predict the draw in Rust, and check after the call that
    exactly one draw was consumed (fallout-rs: 1,200 of 1,200 for the
    sandbox duration).
12. **Finish.** Terminate the process by its own handle, close the job,
    and check the process list for any leftover copy. Report the process
    id, the restart count and the leftover check in the footer.

## Safety rules fallout-rs keeps

- Never resume the main thread; never let the program create a window,
  read a configuration file, load game data or open a network connection.
- Work only on a private copy; refuse to run a program found inside a game
  installation folder; never write into a game folder or a save folder.
- Kill-on-close job from the first instant; terminate by handle (by the
  process's own id, never by name); check for leftovers at the end.
- If a call faults (a fake object was too thin), the remote thread dies,
  the process is restarted from step 1 and the restart is counted in the
  report.
- Everything the tool writes is a private report. Only the derived facts
  (address, rule, counts) leave the machine.

## Limits seen so far

- Functions that need many live engine objects cannot be stood in with
  fakes at reasonable cost. The New Vegas armour step (`009b5a30`, about
  3 KB: both actors, the weapon's mods, the round's effects, three perk
  entry points walking real perk lists, the V.A.T.S. state) is exe-read
  only for that reason.
- No formula needs a running game loop; anything that does (AI ticks,
  physics) is out of reach. Use recordings or nv-rs's `nv-probe` there.
- The control word is a choice. The running game may run with a
  different x87 precision (a Direct3D 9 device without the
  preserve-precision flag sets 24-bit precision). Results that depend on
  precision (NV-001's 880 x 25 case) should be confirmed with the control
  word seen in the running game, which `nv-probe` can log.
- The check is as good as the fakes: a vtable slot answered wrongly gives
  a confident wrong answer. Every check writes its calling convention and
  its liberties (what was faked and how) into the report so a reader can
  judge them.

## What it found in New Vegas

43 rows, 49,159 calls: 39 live checks, 4 not testable (with reasons). Every
mismatch was explained by a proposed reading that matched every sample:
the weapon condition factor (constants 0.75 / 0.67, not the settings),
repair without the skill ceiling, inclusive karma band limits, and the NPC
level's single-precision quotient with a truncated product. The results
are the `oracle-verified` rows of [nv-facts.tsv](nv-facts.tsv). On Fallout 3
the same tool has about 100 rows and some 90,000 calls; its first set of
checks found 9 real errors in fallout-rs's Rust.

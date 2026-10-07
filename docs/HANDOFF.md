# Handoff: overnight session, 2026-10-06 → 07

For the next lead session (Claude). Read [AGENTS.md](../AGENTS.md),
[MILESTONES.md](MILESTONES.md), [CONTRIBUTING.md](../CONTRIBUTING.md) and
[TASKS.md](TASKS.md) first. Your job tonight: fix the playtest bugs and do the
optimizations on the maintainer's list in TASKS.md, using small bounded
agents; push and publish as each fix lands, and keep the integration
branch green.

## State at handoff

Updated 2026-10-07 morning (overnight session 2026-10-06 → 07).

- `main` has everything (merged from `claude/overnight-integration` on
  2026-10-07, PR #26). Work from `main` now; it only changes through pull
  requests (CI checks and the maintainer's review).
- Published: play builds 12 to 23 in `Desktop\nv-rs-play` (23 has everything).
  Every merge was checked with the full root and viewer checks and
  `scripts/acceptance.ps1` before publishing.
- Landed overnight: one radio state and the 2 key (`claude/m2-radio-unify`),
  death into ragdoll (`claude/m2-death-ragdoll`), Chazm's PR #11
  (`claude/contrib-chazm`) and PR #12 (`claude/contrib-chazm-perf`; both in
  docs/CONTRIB_CHAZM.md), B3 crosshair pick, B4 NPCs on the ground, B5
  firing sounds and muzzle flashes, B6 impact decals and sounds, B10
  greetings, B11 voice file names, B12 Doc's door loop, B13 barter over
  dialogue, B16 local map panning (its zoom scale was found to match the
  game already). Details and what each left are in TASKS.md.
- Every overnight branch is merged; nothing is running. Worktrees stay in
  `%USERPROFILE%\nv-re\work\wt-b<n>-*` (their build folders were cleared).
- GitHub: issues #13–#24 are the twelve `[task] M<n>` major systems
  PRs #11 and #12 were closed by PR #26. Contributors' forks: see
  CONTRIBUTING.md, "Already working in a fork?"; what the forks held on
  2026-10-07 is in TASKS.md.
- Ghidra: one shared read-only server replaces per-agent `ghidra.ps1` runs
  for queries (docs/RESEARCH_WORKFLOW.md, "Ghidra MCP trial"). Start it with
  the local `start-nvrs.ps1` in the tool's folder under
  `%USERPROFILE%\nv-re\tools\ghidra-mcp`; agents use its HTTP read
  endpoints on 127.0.0.1:8089. It isn't registered as an MCP server in
  Claude Code yet (the `claude` CLI wasn't on PATH; the snippet is in that
  folder's NVRS-SETUP.md).
- Acceptance flakiness: the gunfight (vms16) failed twice in a row once
  and passed four times in an A/B rerun on the same build; Back in the
  Saddle (vcg02) failed once when Sunny reached the player after the
  route's `StartConversation` and passed two reruns. Both are
  timing-dependent under heavy machine load.
- Agents wrote into the main checkout through relative paths twice
  tonight (both reverted at once); keep checking `git status` there after
  each agent.

## First steps

1. GitHub CLI: `gh` was installed for this session (`C:\Program
   Files\GitHub CLI\gh.exe`; if it isn't there, `winget install
   GitHub.cli`). It needs the user to sign in once (`gh auth login`, in
   their own terminal); never handle their credentials. If `gh auth
   status` fails, ask the user to run it, and use GitHub Desktop's git
   for pushing meanwhile.
2. GitHub tasks: the user may have created the 12 major-system issues
   (TASKS.md M1–M12, titled `[task] M<n>. <name>`) from prefilled links.
   Check with `gh issue list`; create any missing ones with `gh issue
   create` (title and body from TASKS.md, plus the claiming rules from
   CONTRIBUTING.md). Never post the maintainer's list (B-items) as issues.
3. Ghidra MCP trial (one agent, about an hour, in parallel with step 4).
   https://github.com/bethington/ghidra-mcp (Apache 2.0) serves a loaded
   Ghidra program to agents over MCP, so they wouldn't each start a fresh
   8 GB `analyzeHeadless` through `ghidra.ps1` (the main cause of running
   out of memory with several agents, and of the 36 project copies).
   - Pin one release; read its setup script before running it. It targets
     Ghidra 12.1.3; ours is 12.1.4
     (`%USERPROFILE%\nv-re\tools\ghidra_12.1.4_PUBLIC`): check it builds
     and loads.
   - Run its **headless** server on a fresh copy of the analyzed project
     (e.g. copy `ghidra_1` to `%USERPROFILE%\nv-re\ghidra_mcp`), bound to
     127.0.0.1, `GHIDRA_MCP_ALLOW_SCRIPTS` off, and **read-only use only**:
     no renames, types or comments through it (they would change the
     database every agent relies on, with its own naming conventions).
     Naming work (e.g. importing the Xbox prototype names) stays on a
     separate copy.
   - Register it for this project in Claude Code's MCP settings (project
     `.mcp.json` or user settings; don't commit machine paths).
   - Compare: re-trace `00c78610` and `00c755e0` (look-IK) and one
     function from today's batches through it, check the output matches
     our existing exports, and run 2–3 agents against the one server at
     once. Record memory use and time per query against `ghidra.ps1`.
   - Write the result in docs/RESEARCH_WORKFLOW.md. If it holds up, tell
     later agents to use it for Ghidra queries (keeping `ghidra.ps1` and
     the committed `research/` tools as the documented, reproducible
     method contributors can check provenance with), and run fewer project
     copies. If it doesn't, say why and stop.
4. Resume the stopped branches above (at most 3–4 agents at once).
5. Then the maintainer's list.

## Choosing work

1. Work the **maintainer's list** in TASKS.md (B1–B21: the build-11
   playtest bugs and polish). These are ours; they aren't GitHub tasks.
2. Don't start anything in an area claimed on GitHub (`[task] …` issues
   with an assignee or a `Claiming this` comment) or owned by a
   contributor in CONTRIBUTING.md. If a bug fix needs a change there, keep
   it minimal and say so on that issue.
3. When you find a **major system** that's missing (one that would take a
   whole agent-night and isn't in our areas), add it to TASKS.md's
   "Major systems" list and open a `[task]` issue for it, so contributors
   can claim it. Close or update issues as things land.
4. Suggested order: B12 Doc dialogue trap, B13 barter over dialogue, B3
   crosshair pick, B4 NPCs through the ground, B10 greetings, B11 voice
   after skipping, B5/B6 weapon effects and impacts, B14 opening, B1/B2
   physics (large; split), B7, B8, B9, B16, B17, B18–B21.
5. At most 3–4 agents at once (memory: builds with `-j 2`, the lead's
   with `-j 3`; more agents run out of memory and disk).

## Agent workflow (what worked today)

- One worktree and branch per task, created by the lead from the
  integration tip:
  `git worktree add -b claude/<name> %USERPROFILE%\nv-re\work\wt-<name> <integration tip>`.
  Agents must use absolute paths and must never edit the main checkout
  (`Desktop\nv-rs`) or another worktree; they have leaked there before,
  so check `git status` of the main checkout after each agent.
- Each agent gets its own Ghidra project copy
  (`%USERPROFILE%\nv-re\ghidra_N`, set `NVRE_GHIDRA`; scripts in
  `%USERPROFILE%\nv-re\scripts`) and its own `CARGO_TARGET_DIR` for root
  tests (`%USERPROFILE%\nv-re\work\target-<name>`).
- Prompts must require: tracing in Ghidra with provenance, **live
  verification in the release viewer** (screenshots read back; the
  `--key-at SECONDS KEY[:HOLD]` option injects input inside the viewer),
  reporting verified-live separately from unit-tested, the full checks,
  and `scripts/acceptance.ps1`.
- The `repo_hygiene` root test fails in worktrees only (the `.git`
  pointer file). Any other hygiene finding is real.

## Merging, checking, publishing

- Merge each finished branch into integration in its worktree
  (`%USERPROFILE%\nv-re\work\wt-integration`). docs/MILESTONES.md
  conflicts on most merges; `%USERPROFILE%\nv-re\work\union-milestones.ps1`
  (run from the worktree root) keeps both sides. `resolve.ps1 <file>
  o,t,b,...` in the same folder takes ours/theirs/both per conflict.
- After each merge: root `cargo test --workspace`, clippy, fmt; viewer
  tests, clippy, fmt, release build; then
  `powershell -File scripts\acceptance.ps1`. The gunfight is partly
  random: one failure → rerun; two → investigate before publishing
  (compare hit/miss and death lines with a passing log).
- Publish: `powershell -File %USERPROFILE%\nv-re\work\publish-play.ps1
  -Notes "...", "..."` (copies the integration release build into the
  play copy, writes WHATS-NEW.txt). Push integration afterwards.
- Git isn't on PATH: use GitHub Desktop's
  (`%LOCALAPPDATA%\GitHubDesktop\app-3.6.6\resources\app\git\cmd\git.exe`).
  `gh` (GitHub CLI) is installed once the user has signed it in; use it for
  issues and pull requests (open PRs against `main` for contributors'
  review when the user asks).

## Rules to keep

- No stand-ins: nothing ships that isn't traced or recorded; untraced
  behaviour goes behind `NV_GUESSES=1` (`world::guesses`) or stays out.
- Never commit game files, exe bytes, decompiler output, recordings or
  logs; keep them under `%USERPROFILE%\nv-re`.
- Don't touch AGENTS.md. Don't merge into `main`.
- Whole systems, not patches: when fixing a bug, finish the system it
  belongs to as the game has it.

## Morning report

Update docs/MILESTONES.md and this file's "State at handoff", update any
GitHub issues touched (what landed, what's left), and leave the
user a short summary: builds published, tasks done, what's verified
live, what isn't, and anything that needs their decision.

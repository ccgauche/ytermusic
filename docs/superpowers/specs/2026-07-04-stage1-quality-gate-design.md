# Stage 1 Quality Gate Design

## Goal

Preserve the current lyrics, visualizer, gauge, and player-test work while restoring a reliable repository quality gate. Completion requires formatting, Clippy, and all workspace tests to pass without warnings.

## Scope

Stage 1 includes only build and test hygiene needed to make the current work safely reviewable:

- preserve all tracked and untracked changes from the original checkout in an isolated worktree;
- repair the new player tests against the public `player` API;
- relocate the three `test_*` prototype binaries so Cargo does not treat them as production executables;
- format the current Rust sources and remove warnings introduced by the work in progress;
- move the ignored release profile from the member manifest to the workspace root;
- make pull-request CI run the same workspace-wide checks used locally.

The lyrics request lifecycle, downloader retry behavior, authentication logging, persistence hardening, responsive layout, and real audio visualization belong to later stages. Stage 1 may add characterization tests that expose those defects, but it will not mix their production fixes into this change.

## Isolation and Change Ownership

Work happens on branch `codex/stage1-quality-gate` in `/tmp/ytermusic-worktrees/stage1-quality-gate`. The original checkout at `/home/itthiago/ytermusic` remains unchanged.

The worktree begins with an exact copy of the original checkout's modified and untracked files. These pre-existing changes remain user-owned. Stage 1 edits only the files required by this specification and reports the resulting diff before any integration decision.

## Test Structure

`crates/player/tests/player.rs` remains an integration test and imports the public `player::{Player, PlayerOptions}` API. Tests construct options through `PlayerOptions::new(initial_volume)` rather than private fields or crate-private constants. Assertions cover public behavior that can run without opening media files or requiring a real audio device wherever the existing player abstraction permits it.

The prototype files currently under `crates/ytermusic/src/bin/` are not production commands:

- reusable deterministic checks become automated tests near the code they exercise;
- interactive or environment-dependent experiments move to `examples/` only if they still provide diagnostic value;
- machine-specific paths and personal configuration assumptions are removed.

No automated test may require a real YouTube account, browser cookies, network access, or a developer-specific absolute path.

## CI Contract

The pull-request workflow becomes the canonical quality gate and runs:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

The commands use the checked-in lockfile. Linux system dependencies remain installed before Rust commands. Existing manual platform workflows may remain for cross-platform diagnostics, but they do not substitute for the pull-request test gate.

## Error and Warning Policy

Stage 1 does not suppress warnings with broad `allow` attributes. Unused imports, dead prototype code, formatting drift, and invalid tests are removed or corrected at their source.

If an existing test requires credentials or network access, it must be marked as an explicit ignored integration test with documented prerequisites or replaced by deterministic fixtures. The default workspace test command must be hermetic.

## Validation

Validation occurs in this order:

1. Run each repaired or newly relocated test in isolation.
2. Run the affected crate's complete test suite.
3. Run the three canonical workspace commands from the CI contract.
4. Inspect `git diff --check` and `git status --short`.

The stage is complete only when all commands exit successfully and the output contains no compiler or Clippy warnings.

## Deliverable

The deliverable is a reviewable Stage 1 commit on the isolated task branch. Integration into the original checkout is a separate user-approved action after the diff and validation evidence are presented.

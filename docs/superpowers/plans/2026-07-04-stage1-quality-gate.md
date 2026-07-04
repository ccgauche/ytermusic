# Stage 1 Quality Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the current local interface work while making the complete Rust workspace format-clean, warning-free, hermetically testable, and enforced by pull-request CI.

**Architecture:** Keep production behavior unchanged in Stage 1. Replace device- and credential-dependent default tests with pure public-API tests, retain the live API probe as a non-installed example, move the release profile to the virtual workspace, and make CI execute the same canonical commands used locally.

**Tech Stack:** Rust 2021/2024 workspace, Cargo, rustfmt, Clippy, GitHub Actions, Ratatui, Rodio, Tokio.

---

### Task 1: Record the failing baseline

**Files:**
- Inspect: `crates/player/tests/player.rs`
- Inspect: `crates/ytermusic/src/bin/*.rs`
- Inspect: `.github/workflows/ci.yml`

- [ ] **Step 1: Reproduce the formatting failure**

Run:

```bash
cargo fmt --all -- --check
```

Expected: FAIL with diffs in the imported player tests and TUI work.

- [ ] **Step 2: Reproduce the test-compilation failure**

Run:

```bash
cargo test --workspace --all-targets --locked --offline
```

Expected: FAIL because `crates/player/tests/player.rs` uses `super::*`, private constants, nonexistent `PlayerOptions` fields, and an incorrect `Player::new` return shape.

- [ ] **Step 3: Reproduce the lint failure**

Run:

```bash
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Expected: FAIL on unused imports and variables in the prototype binaries and current TUI work.

### Task 2: Make player tests public-API and device independent

**Files:**
- Modify: `crates/player/tests/player.rs`

- [ ] **Step 1: Replace invalid device-dependent tests with public option tests**

Use the public constructor and accessors only:

```rust
use player::PlayerOptions;

#[test]
fn preserves_valid_initial_volume() {
    let options = PlayerOptions::new(40);

    assert_eq!(options.initial_volume(), 40);
    assert_eq!(options.initial_volume_f32(), 0.4);
}

#[test]
fn clamps_initial_volume_to_one_hundred() {
    let options = PlayerOptions::new(150);

    assert_eq!(options.initial_volume(), 100);
    assert_eq!(options.initial_volume_f32(), 1.0);
}
```

Do not construct `Player`: its public constructor opens a real audio device, which is not hermetic in CI.

- [ ] **Step 2: Run the player integration test**

Run:

```bash
cargo test -p player --test player --locked --offline
```

Expected: PASS, 2 tests.

### Task 3: Isolate live API tests from the default suite

**Files:**
- Modify: `crates/ytpapi2/src/lib.rs:28-108`
- Move: `crates/ytermusic/src/bin/test_api.rs` to `crates/ytermusic/examples/api_probe.rs`

- [ ] **Step 1: Require an explicit path for credentialed tests**

Replace implicit `../headers.txt` reads with a test-only helper based on `YTERMUSIC_HEADERS_PATH`:

```rust
#[cfg(test)]
fn live_headers_path() -> std::path::PathBuf {
    std::env::var_os("YTERMUSIC_HEADERS_PATH")
        .map(std::path::PathBuf::from)
        .expect("set YTERMUSIC_HEADERS_PATH to run ignored live API tests")
}

#[cfg(test)]
fn get_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    let file = std::fs::read_to_string(live_headers_path())
        .expect("failed to read YTERMUSIC_HEADERS_PATH");
    for header in file.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = header
            .split_once(": ")
            .expect("live test header lines must contain ': '");
        let name = match key {
            "Cookie" => reqwest::header::COOKIE,
            "User-Agent" => reqwest::header::USER_AGENT,
            _ => continue,
        };
        headers.insert(name, value.parse().expect("invalid live test header value"));
    }
    headers
}

#[cfg(test)]
fn get_account_id() -> Option<String> {
    let headers_path = live_headers_path();
    let account_path = headers_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("account_id.txt");
    std::fs::read_to_string(account_path)
        .ok()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
}
```

Read `account_id.txt` from the selected header file's parent and return `None` when absent. Mark each credentialed network test with:

```rust
#[ignore = "requires YTERMUSIC_HEADERS_PATH and live YouTube Music access"]
```

Use `assert!(!value.is_empty())` rather than boolean comparisons so Clippy remains clean.

- [ ] **Step 2: Convert the API prototype into a non-installed example**

Move it to `crates/ytermusic/examples/api_probe.rs`. Accept the header path as argument one, print a usage message and exit with status 2 when absent, and handle `YoutubeMusicInstance::from_header_file` without `unwrap()`.

```rust
use std::path::PathBuf;
use ytpapi2::YoutubeMusicInstance;

#[tokio::main]
async fn main() {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run -p ytermusic --example api_probe -- <headers-path>");
        std::process::exit(2);
    };
    let api = match YoutubeMusicInstance::from_header_file(&path).await {
        Ok(api) => api,
        Err(error) => {
            eprintln!("failed to initialize API: {error:?}");
            std::process::exit(1);
        }
    };
    match api.search("rock", 0).await {
        Ok(result) => println!(
            "search succeeded: {} videos, {} playlists",
            result.videos.len(),
            result.playlists.len()
        ),
        Err(error) => {
            eprintln!("search failed: {error:?}");
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 3: Verify default API tests are hermetic**

Run:

```bash
env -u YTERMUSIC_HEADERS_PATH cargo test -p ytpapi2 --locked --offline
```

Expected: PASS with the three live tests reported as ignored and no filesystem or network access.

### Task 4: Convert UI/config prototypes into deterministic tests

**Files:**
- Delete: `crates/ytermusic/src/bin/test_toml.rs`
- Delete: `crates/ytermusic/src/bin/test_tui.rs`
- Modify: `crates/ytermusic/src/config.rs`
- Modify: `crates/ytermusic/src/utils.rs`

- [ ] **Step 1: Add a config serialization characterization test**

Add a `#[cfg(test)] mod tests` in `config.rs` that serializes `Config::default()` with `toml::to_string_pretty`, parses it back into `Config`, and asserts the key default fields such as `initial_volume`, `parallel_downloads`, and `volume_slider` survive the round trip.

```rust
#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_config_round_trips_through_toml() {
        let original = Config::default();
        let serialized = toml::to_string_pretty(&original).expect("serialize default config");
        let decoded: Config = toml::from_str(&serialized).expect("deserialize default config");

        assert_eq!(decoded.global.parallel_downloads, original.global.parallel_downloads);
        assert_eq!(decoded.player.initial_volume, original.player.initial_volume);
        assert_eq!(decoded.player.volume_slider, original.player.volume_slider);
    }
}
```

- [ ] **Step 2: Add deterministic gradient characterization tests**

Add tests in `utils.rs` that assert:

```rust
assert_eq!(
    interpolate_color(Color::Rgb(0, 10, 20), Color::Rgb(100, 110, 120), 0.5),
    Color::Rgb(50, 60, 70)
);
```

Create a small `Buffer`, call `fill_gradient_bg`, and assert cells in the same row receive the same background and later rows differ from the first row.

```rust
#[test]
fn fills_each_row_with_a_gradient_color() {
    let area = Rect::new(0, 0, 2, 3);
    let mut buffer = Buffer::empty(area);

    fill_gradient_bg(
        &mut buffer,
        area,
        Color::Rgb(0, 0, 0),
        Color::Rgb(90, 90, 90),
    );

    assert_eq!(buffer.get(0, 0).bg, buffer.get(1, 0).bg);
    assert_ne!(buffer.get(0, 0).bg, buffer.get(0, 2).bg);
}
```

- [ ] **Step 3: Remove the superseded prototype binaries**

Delete `test_toml.rs` and `test_tui.rs`; their useful deterministic behavior is now covered by unit tests and Cargo no longer registers them as installable commands.

- [ ] **Step 4: Run the ytermusic test targets**

Run:

```bash
cargo test -p ytermusic --all-targets --locked --offline
```

Expected: PASS without requiring headers, network access, or an audio device.

### Task 5: Correct workspace build configuration

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/ytermusic/Cargo.toml`

- [ ] **Step 1: Move the release profile to the workspace root**

Remove `[profile.release]` from `crates/ytermusic/Cargo.toml` and add this block to the root `Cargo.toml`:

```toml
[profile.release]
codegen-units = 1
debug = true
lto = true
opt-level = 3
```

- [ ] **Step 2: Verify Cargo accepts the profile without warnings**

Run:

```bash
cargo metadata --no-deps --format-version 1 --locked --offline
```

Expected: exit 0 with no `profiles for the non root package will be ignored` warning.

### Task 6: Enforce the canonical pull-request gate

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Replace the partial commands with canonical workspace commands**

Keep system dependency and toolchain setup, then run:

```yaml
      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run Clippy
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: Run tests
        run: cargo test --workspace --all-targets --locked
```

Remove the redundant plain `cargo check`; Clippy already type-checks every target and the test command builds and runs them.

- [ ] **Step 2: Inspect the workflow diff**

Run:

```bash
git diff --check -- .github/workflows/ci.yml
```

Expected: exit 0.

### Task 7: Format and resolve warnings at their source

**Files:**
- Modify: current Rust files reported by rustfmt or Clippy

- [ ] **Step 1: Apply canonical formatting**

Run:

```bash
cargo fmt --all
```

- [ ] **Step 2: Run Clippy and fix only reported Stage 1 warnings**

Run:

```bash
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Expected initially: any remaining warnings are actionable. Remove unused imports, simplify test assertions, and correct warning sources without adding broad `#[allow]` attributes or changing feature behavior. Re-run until exit 0.

### Task 8: Run the complete quality gate

**Files:**
- Inspect: complete worktree

- [ ] **Step 1: Verify formatting**

Run:

```bash
cargo fmt --all -- --check
```

Expected: exit 0.

- [ ] **Step 2: Verify Clippy**

Run:

```bash
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

Expected: exit 0 without warnings.

- [ ] **Step 3: Verify all tests**

Run:

```bash
cargo test --workspace --all-targets --locked --offline
```

Expected: exit 0; live API tests ignored; all deterministic tests pass.

- [ ] **Step 4: Verify patch hygiene**

Run:

```bash
git diff --check
git status --short --branch
```

Expected: no whitespace errors; only intended imported work and Stage 1 files are listed.

- [ ] **Step 5: Commit the validated Stage 1 result**

Stage all intended files after reviewing `git diff --stat` and commit with:

```bash
git commit -m "chore: restore workspace quality gate"
```

Do not integrate or push. Present the branch, complete diff summary, commit IDs, and verification output for user review.

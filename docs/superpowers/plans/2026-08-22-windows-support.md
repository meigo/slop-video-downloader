# Windows Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Windows `.exe`/`.msi` installers from CI and remove the Unix-only assumptions that would make the app unusable on Windows.

**Architecture:** Six independent changes. CI gains a Windows matrix entry and a split create-release job. Four Rust files lose Unix assumptions: tool lookup (`which`→`where`), console-window suppression on child processes, the save directory, and platform hint strings. The frontend renders hint strings supplied by the backend rather than detecting the OS itself.

**Tech Stack:** Rust (Tauri 2), Svelte 5, GitHub Actions, `tauri-apps/tauri-action@v0`

**Spec:** `docs/superpowers/specs/2026-08-22-windows-support-design.md`

## Global Constraints

- **Keep `APPLE_SIGNING_IDENTITY: '-'`** in the workflow (spec W2). The compositor omits it; copying that omission regresses the macOS first-launch flow.
- **No new frontend dependencies** (spec W6). Platform text comes from the backend, not an OS-detection plugin.
- **No new Rust crate dependencies.** `dirs` is already present and is the only one needed.
- **macOS behaviour must not change.** Every `cfg` arm added must leave the existing macOS path byte-identical in effect.
- **Do not claim Windows support in the README as "tested"** (spec Non-goals). Wording must say builds are provided and untested by the author.
- Tests must stay green: **46 Rust**, **22 frontend**, `npm run check` 0 errors, `cargo build` 0 warnings.
- Run `cargo` commands from `src-tauri/`, `npm` commands from the repo root.

---

### Task 1: CI — Windows target and split release job

Implements spec W1, W2, W8. Ordered first because the current code *compiles* on Windows (`which` fails at runtime, not build time), so this proves the toolchain and NSIS bundler before any Rust work.

**Files:**
- Modify: `.github/workflows/release.yml` (replace entire file)

**Interfaces:**
- Consumes: nothing.
- Produces: a `workflow_dispatch` trigger later tasks use to re-verify Windows builds, and artifacts named `*_x64-setup.exe` / `*_x64_en-US.msi`.

- [ ] **Step 1: Replace the workflow file**

Write `.github/workflows/release.yml` with exactly this content:

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'
  # Build-only smoke test: no tag, no release, nothing uploaded.
  workflow_dispatch:

permissions:
  contents: write

jobs:
  create-release:
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    outputs:
      release_id: ${{ steps.create.outputs.result }}
    steps:
      - uses: actions/checkout@v4

      - name: Verify versions match the tag
        run: |
          tag="${GITHUB_REF_NAME#v}"
          for entry in \
            "src-tauri/tauri.conf.json:$(jq -r .version src-tauri/tauri.conf.json)" \
            "package.json:$(jq -r .version package.json)" \
            "src-tauri/Cargo.toml:$(grep -m1 '^version = ' src-tauri/Cargo.toml | cut -d'"' -f2)"
          do
            if [ "${entry#*:}" != "$tag" ]; then
              echo "::error::${entry%%:*} is at version ${entry#*:}, but the tag is $GITHUB_REF_NAME"
              exit 1
            fi
          done

      - uses: actions/github-script@v9
        id: create
        env:
          RELEASE_BODY: |
            ### Which download do I need?

            **macOS**

            - **`_aarch64.dmg`** — Macs with an Apple chip (M1/M2/M3/M4, from 2020 onwards)
            - **`_x64.dmg`** — older Macs with an Intel processor

            Not sure? Apple menu → **About This Mac**: "Chip: Apple M…" means Apple Silicon,
            "Processor: Intel…" means Intel. The `.app.tar.gz` files are the same apps
            without an installer — most users want the `.dmg`.

            **Windows** (64-bit)

            - **`_x64-setup.exe`** — the installer most people want
            - **`_x64_en-US.msi`** — the same app, for managed or scripted installs

            ### Requirements

            Requires `yt-dlp` and `ffmpeg` on your PATH:

            - macOS — `brew install yt-dlp ffmpeg`
            - Windows — `winget install yt-dlp.yt-dlp` and `winget install Gyan.FFmpeg`

            **Keep yt-dlp current.** YouTube breaks older builds regularly, and when it
            does the app fails to download even though it looks fine otherwise. If a video
            errors out, update first:

            - macOS — `brew upgrade yt-dlp`
            - Windows — `winget upgrade yt-dlp.yt-dlp`

            The app warns you when its yt-dlp is more than two months old.

            ### First launch

            **macOS** — the app is not signed with an Apple Developer ID, so macOS blocks it
            the first time. Drag it to Applications, then either:

            - run `xattr -cr "/Applications/Slop Video Downloader.app"` in Terminal
              and open it normally, or
            - try to open it once, then go to **System Settings → Privacy & Security**
              and click **Open Anyway**.

            Right-click → Open does not work for unsigned apps.

            **Windows** — the installer is not signed: SmartScreen may warn,
            choose **More info → Run anyway**.
        with:
          result-encoding: string
          script: |
            const { data } = await github.rest.repos.createRelease({
              owner: context.repo.owner,
              repo: context.repo.repo,
              tag_name: process.env.GITHUB_REF_NAME,
              name: `Slop Video Downloader ${process.env.GITHUB_REF_NAME}`,
              body: process.env.RELEASE_BODY,
              draft: true,
            });
            return data.id;

  build:
    needs: create-release
    # create-release is skipped on workflow_dispatch; still build then.
    if: ${{ !cancelled() && needs.create-release.result != 'failure' }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - { os: macos-latest, target: aarch64-apple-darwin }
          - { os: macos-latest, target: x86_64-apple-darwin }
          - { os: windows-latest, target: x86_64-pc-windows-msvc }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - run: npm ci

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          # Ad-hoc signature: seals the .app bundle so macOS stops calling it
          # "damaged". Not a Developer ID, so Gatekeeper still needs a bypass.
          # No effect on Windows.
          APPLE_SIGNING_IDENTITY: '-'
        with:
          releaseId: ${{ needs.create-release.outputs.release_id }}
          args: --target ${{ matrix.target }}
```

- [ ] **Step 2: Validate the YAML parses**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('valid')"`
Expected: `valid`

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: build Windows installers and split the release-creation job"
```

- [ ] **Step 4: Push and run the build-only smoke test**

```bash
git push origin main
gh workflow run release.yml --ref main
```

- [ ] **Step 5: Verify all three targets build**

Run: `gh run list --workflow=release.yml --limit 1`
Wait for `completed success`. Then:

Run: `gh run view --log --job "$(gh run list --workflow=release.yml --limit 1 --json databaseId --jq '.[0].databaseId')" 2>/dev/null | grep -iE "setup.exe|\.msi|error" | head`

Expected: the Windows job succeeds and mentions `_x64-setup.exe` and `_x64_en-US.msi`. No release is created (this is `workflow_dispatch`).

**If the Windows job fails here, stop and report.** It means the NSIS bundler needs configuration, which is a change to `src-tauri/tauri.conf.json` not covered by this plan.

---

### Task 2: `proc.rs` — suppress child console windows on Windows

Implements spec W5. A GUI-subsystem process spawning a console process makes Windows allocate a console window per child. The app shells out on every user action, so without this a black window blinks repeatedly.

**Files:**
- Create: `src-tauri/src/proc.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod proc;`)
- Modify: `src-tauri/src/ytdlp.rs` (4 spawn sites)
- Modify: `src-tauri/src/export.rs` (2 spawn sites)
- Modify: `src-tauri/src/deps.rs` (1 spawn site)

**Interfaces:**
- Consumes: nothing.
- Produces: `crate::proc::command(bin: &str) -> std::process::Command` — used by Task 3.

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/proc.rs` containing only the test module:

```rust
//! Spawning external tools.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_keeps_the_program_name() {
        let c = command("yt-dlp");
        assert_eq!(c.get_program(), "yt-dlp");
    }
}
```

Add to `src-tauri/src/lib.rs` after the line `mod export;`:

```rust
mod proc;
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test proc::`
Expected: FAIL — `cannot find function 'command' in this scope`.

- [ ] **Step 3: Write the implementation**

Put this **above** the `#[cfg(test)]` block in `src-tauri/src/proc.rs`:

```rust
//! Spawning external tools.

use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// A GUI-subsystem process makes Windows allocate a console window for every
/// console child it spawns. `main.rs` sets `windows_subsystem = "windows"` for
/// the app itself, but that does not cover children — so yt-dlp and ffmpeg
/// would each flash a black window on every user action.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Build a `Command` for an external tool, without a visible console on Windows.
///
/// Use for every tool the app runs on the user's behalf. Do **not** use for
/// `open`/`explorer`/`xdg-open` — those are meant to show a window.
pub fn command(bin: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(bin);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd src-tauri && cargo test proc::`
Expected: PASS.

- [ ] **Step 5: Route every tool spawn through the helper**

In `src-tauri/src/ytdlp.rs`, replace `Command::new("yt-dlp")` with `crate::proc::command("yt-dlp")` at **all four** sites:
- inside `installed_ytdlp_version` (the `--version` call)
- inside `fetch_metadata`
- inside `resolve_preview`'s stream-URL block
- inside `resolve_preview`'s preview-download block

In `src-tauri/src/export.rs`, two sites:

```rust
fn run_ytdlp(args: Vec<String>) -> Result<std::process::Output, String> {
    crate::proc::command("yt-dlp")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))
}
```

and

```rust
        crate::proc::command("ffmpeg").args(args).output()
```

In `src-tauri/src/deps.rs`, one site inside `which`:

```rust
    let output = crate::proc::command("which").arg(bin).output().ok()?;
```

Leave `settings.rs` untouched — its `open`/`explorer`/`xdg-open` calls are supposed to open a window.

- [ ] **Step 6: Verify no tool spawn bypasses the helper**

Run: `cd src-tauri && grep -rn 'Command::new("yt-dlp")\|Command::new("ffmpeg")\|Command::new("which")' src/`
Expected: no output.

Run: `cd src-tauri && cargo test 2>&1 | grep "test result"`
Expected: `47 passed` (46 existing + the new one).

Run: `cd src-tauri && cargo build 2>&1 | grep -c '^warning'`
Expected: `0`

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/proc.rs src-tauri/src/lib.rs src-tauri/src/ytdlp.rs src-tauri/src/export.rs src-tauri/src/deps.rs
git commit -m "fix: suppress child console windows on Windows"
```

---

### Task 3: `deps.rs` — cross-platform tool lookup

Implements spec W3, W4. `which` does not exist on Windows, so today every dep check returns "not found" and the app would sit on the MissingDeps blocker forever.

**Files:**
- Modify: `src-tauri/src/deps.rs`

**Interfaces:**
- Consumes: `crate::proc::command` from Task 2.
- Produces: `first_path_line(stdout: &[u8]) -> Option<String>` (private, tested).

- [ ] **Step 1: Write the failing test**

Add to the `mod tests` block in `src-tauri/src/deps.rs`:

```rust
    #[test]
    fn first_path_line_takes_one_result() {
        // `where` prints every match, one per line; `which` prints one.
        let many = b"C:\\tools\\yt-dlp.exe\r\nC:\\other\\yt-dlp.exe\r\n";
        assert_eq!(
            first_path_line(many).as_deref(),
            Some("C:\\tools\\yt-dlp.exe")
        );
        assert_eq!(
            first_path_line(b"/opt/homebrew/bin/yt-dlp\n").as_deref(),
            Some("/opt/homebrew/bin/yt-dlp")
        );
    }

    #[test]
    fn first_path_line_rejects_empty_output() {
        assert_eq!(first_path_line(b""), None);
        assert_eq!(first_path_line(b"   \n"), None);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test first_path_line`
Expected: FAIL — `cannot find function 'first_path_line' in this scope`.

- [ ] **Step 3: Write the implementation**

Replace the whole `which` function in `src-tauri/src/deps.rs` with:

```rust
/// First line of stdout from a path lookup, trimmed. `where` prints every
/// match, so only the first is meaningful.
fn first_path_line(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    let line = text.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(line.to_string())
    }
}

/// Locate `bin` on PATH. Windows has no `which`; the equivalent is `where`.
fn which(bin: &str) -> Option<String> {
    #[cfg(windows)]
    let lookup = "where";
    #[cfg(not(windows))]
    let lookup = "which";

    let output = crate::proc::command(lookup).arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    first_path_line(&output.stdout)
}
```

- [ ] **Step 4: Gate the Unix-only PATH augmentation**

`TOOL_DIRS` and `augmented_path` reference Homebrew and MacPorts paths that do not exist on Windows. In `src-tauri/src/deps.rs`, add `#[cfg(unix)]` immediately above both:

- `const TOOL_DIRS: [&str; 3] = ...`
- `fn augmented_path(...)`

Then replace `pub fn ensure_tool_path()` with this pair, so callers keep one name on both platforms:

```rust
/// Must run before any tool is spawned: `Command` inherits this process's PATH.
#[cfg(unix)]
pub fn ensure_tool_path() {
    let current = std::env::var("PATH").unwrap_or_default();
    let home = std::env::var("HOME").ok();
    std::env::set_var("PATH", augmented_path(&current, home.as_deref()));
}

/// Windows installers put tools on the machine PATH already, and the Unix
/// well-known directories do not exist there.
#[cfg(not(unix))]
pub fn ensure_tool_path() {}
```

- [ ] **Step 5: Gate the Unix-only tests**

Add `#[cfg(unix)]` immediately above each of these three existing tests in `src-tauri/src/deps.rs`:
- `prepends_tool_dirs_to_launchd_default`
- `includes_user_local_bin_when_home_known`
- `keeps_existing_entries_once`

- [ ] **Step 6: Run the tests**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result"`
Expected: `49 passed` (47 from Task 2 + 2 new).

Run: `cd src-tauri && cargo build 2>&1 | grep -c '^warning'`
Expected: `0`

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/deps.rs
git commit -m "fix: locate yt-dlp and ffmpeg with 'where' on Windows"
```

---

### Task 4: Cross-platform default save directory

Implements spec W7. `default_save_dir` returns `~/Movies/Slop Refs`; Windows has no `Movies` folder.

**Files:**
- Modify: `src-tauri/src/settings.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: no signature change — `default_save_dir() -> String` as before.

- [ ] **Step 1: Write the failing test**

Add to the existing `mod tests` block in `src-tauri/src/settings.rs` (it starts at line 96):

```rust
    #[test]
    fn default_save_dir_is_absolute_and_named() {
        let d = default_save_dir();
        assert!(!d.is_empty(), "save dir must not be empty");
        assert!(
            std::path::Path::new(&d).is_absolute(),
            "save dir must be absolute, got {d}"
        );
        assert!(d.ends_with("Slop Refs"), "save dir must end in Slop Refs, got {d}");
        // The macOS-only "Movies" literal must not be hardcoded any more.
        #[cfg(windows)]
        assert!(!d.contains("Movies"), "Windows must not use a Movies folder");
    }
```

- [ ] **Step 2: Run the test to verify it fails on Windows semantics**

Run: `cd src-tauri && cargo test default_save_dir_is_absolute_and_named`
Expected: PASS on macOS (the current code already satisfies it there). This test is a **regression guard**; it fails on Windows CI in Step 5 if the implementation is not fixed.

- [ ] **Step 3: Write the implementation**

Replace `default_save_dir` in `src-tauri/src/settings.rs` with:

```rust
pub fn default_save_dir() -> String {
    // ~/Movies/Slop Refs on macOS, %USERPROFILE%\Videos\Slop Refs on Windows.
    let base = dirs::video_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default());
    base.join("Slop Refs").to_string_lossy().into()
}
```

- [ ] **Step 4: Run the tests**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result"`
Expected: `50 passed`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/settings.rs
git commit -m "fix: default to the platform videos folder for saved clips"
```

---

### Task 5: Platform-aware install and upgrade hints

Implements spec W6. `MissingDeps.svelte` hardcodes `brew install yt-dlp ffmpeg` and the staleness banner hardcodes `brew upgrade yt-dlp`; both are wrong on Windows. The backend knows the platform at compile time, so it supplies the strings.

**Files:**
- Modify: `src-tauri/src/deps.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/components/MissingDeps.svelte:33`
- Modify: `src/routes/+page.svelte` (the `.stale` banner and the `refreshDeps` error fallback)

**Interfaces:**
- Consumes: `DepsStatus` from Task 3.
- Produces: `DepsStatus.install_hint: String`, `DepsStatus.upgrade_hint: String`; TypeScript `install_hint: string`, `upgrade_hint: string`.

- [ ] **Step 1: Write the failing test**

Add to the `mod tests` block in `src-tauri/src/deps.rs`:

```rust
    #[test]
    fn hints_name_this_platform_package_manager() {
        #[cfg(target_os = "macos")]
        {
            assert!(install_hint().contains("brew install"));
            assert!(upgrade_hint().contains("brew upgrade"));
        }
        #[cfg(windows)]
        {
            assert!(install_hint().contains("winget install"));
            assert!(upgrade_hint().contains("winget upgrade"));
        }
        // Both must name yt-dlp on every platform.
        assert!(install_hint().contains("yt-dlp"));
        assert!(upgrade_hint().contains("yt-dlp"));
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd src-tauri && cargo test hints_name_this_platform`
Expected: FAIL — `cannot find function 'install_hint' in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `src-tauri/src/deps.rs`, above `check_deps`:

```rust
/// Command that installs both tools on this platform.
pub fn install_hint() -> &'static str {
    #[cfg(windows)]
    {
        "winget install yt-dlp.yt-dlp Gyan.FFmpeg"
    }
    #[cfg(not(windows))]
    {
        "brew install yt-dlp ffmpeg"
    }
}

/// Command that updates yt-dlp on this platform.
pub fn upgrade_hint() -> &'static str {
    #[cfg(windows)]
    {
        "winget upgrade yt-dlp.yt-dlp"
    }
    #[cfg(not(windows))]
    {
        "brew upgrade yt-dlp"
    }
}
```

Add the two fields to the `DepsStatus` struct, after `ytdlp_stale`:

```rust
    /// Platform-correct install command, e.g. `brew install yt-dlp ffmpeg`.
    pub install_hint: String,
    /// Platform-correct yt-dlp update command, e.g. `brew upgrade yt-dlp`.
    pub upgrade_hint: String,
```

And populate them in the `DepsStatus { ... }` literal inside `check_deps`, after `ytdlp_stale,`:

```rust
        install_hint: install_hint().to_string(),
        upgrade_hint: upgrade_hint().to_string(),
```

- [ ] **Step 4: Run the Rust tests**

Run: `cd src-tauri && cargo test 2>&1 | grep "test result"`
Expected: `51 passed`.

- [ ] **Step 5: Mirror the fields in TypeScript**

In `src/lib/types.ts`, add to `DepsStatus` after `ytdlp_stale: boolean;`:

```typescript
  install_hint: string;
  upgrade_hint: string;
```

- [ ] **Step 6: Render the hints instead of literals**

In `src/lib/components/MissingDeps.svelte`, replace line 33:

```svelte
      <code>{deps.install_hint}</code>
```

In `src/routes/+page.svelte`, replace the `<code>` inside the `.stale` banner:

```svelte
            yt-dlp {deps.ytdlp_version} is over two months old — YouTube regularly breaks
            older builds. Update with <code>{deps.upgrade_hint}</code>.
```

In the same file, the `refreshDeps` catch block builds a fallback `DepsStatus`; add the two fields so the object still typechecks:

```typescript
      deps = {
        ytdlp: false,
        ffmpeg: false,
        ytdlp_path: null,
        ffmpeg_path: null,
        ytdlp_version: null,
        ytdlp_stale: false,
        install_hint: "brew install yt-dlp ffmpeg",
        upgrade_hint: "brew upgrade yt-dlp",
      };
```

- [ ] **Step 7: Verify the frontend**

Run: `npm run check 2>&1 | tail -1`
Expected: `0 ERRORS 0 WARNINGS`

Run: `npm test 2>&1 | grep "Tests "`
Expected: `22 passed`

Run: `grep -rn "brew install\|brew upgrade" src/lib/components/MissingDeps.svelte src/routes/+page.svelte`
Expected: only the two fallback strings inside `refreshDeps` — no literals in markup.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/deps.rs src/lib/types.ts src/lib/components/MissingDeps.svelte src/routes/+page.svelte
git commit -m "feat: name the right package manager per platform in dep hints"
```

---

### Task 6: README

Implements spec component 6. `README.md:13` currently states the app is macOS-only and that spawning is Unix-only, which is no longer true.

**Files:**
- Modify: `README.md:13` (platforms line) and `README.md:39` (install command)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing.

- [ ] **Step 1: Replace the platforms line**

Replace line 13 of `README.md` with:

```markdown
**Platforms:** macOS (Apple Silicon and Intel) and Windows (64-bit) — see the
[releases page](https://github.com/meigo/slop-video-downloader/releases). Windows builds are
produced by CI but have not been exercised by the author; please report anything broken.
```

- [ ] **Step 2: Add Windows requirements**

`README.md:39` holds `brew install yt-dlp ffmpeg` inside a fenced block. Directly after that block, add:

```markdown
On Windows:

```
winget install yt-dlp.yt-dlp
winget install Gyan.FFmpeg
```

The installer is unsigned, so SmartScreen may warn — choose **More info → Run anyway**.
```

- [ ] **Step 3: Verify no stale claim remains**

Run: `grep -niE "macos only|unix-only|todo list" README.md`
Expected: no output.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: document Windows builds and requirements"
```

---

### Task 7: Verify the full matrix and cut a release

**Files:** none modified.

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Push and run the smoke test again**

```bash
git push origin main
gh workflow run release.yml --ref main
```

- [ ] **Step 2: Confirm all three targets pass with the code changes in**

Run: `gh run list --workflow=release.yml --limit 1`
Expected: `completed success`. The Windows job now also runs `cargo test`, which is the first execution of the `cfg(windows)` arms — a compile error there means a `cfg` mistake macOS could not catch.

- [ ] **Step 3: Bump the version**

```bash
npm version 0.2.0 --no-git-tag-version
sed -i '' '3s/0\.1\.3/0.2.0/' src-tauri/Cargo.toml
sed -i '' '4s/0\.1\.3/0.2.0/' src-tauri/tauri.conf.json
cd src-tauri && cargo update -p slop-video-downloader --precise 0.2.0 && cd ..
```

- [ ] **Step 4: Verify every version matches**

Run:
```bash
grep -m1 '"version"' package.json; grep -m1 '^version' src-tauri/Cargo.toml; grep -m1 '"version"' src-tauri/tauri.conf.json
```
Expected: all three read `0.2.0`. (Task 1's workflow check enforces this at tag time, but catching it here avoids a failed release run.)

- [ ] **Step 5: Commit and tag**

```bash
git add -A
git commit -m "chore: release 0.2.0"
git push origin main
git tag v0.2.0
git push origin v0.2.0
```

- [ ] **Step 6: Confirm the draft release has six artifacts**

Run: `gh release view v0.2.0 --json isDraft,assets --jq '{draft:.isDraft, assets:[.assets[].name]}'`
Expected: `draft: true`, with four macOS artifacts plus `_x64-setup.exe` and `_x64_en-US.msi`.

- [ ] **Step 7: STOP — hand off for Windows testing**

**Do not publish the release.** Per the spec's Definition of Done, four things need a human on real Windows first:

1. yt-dlp and ffmpeg resolve (no MissingDeps blocker when they are installed)
2. Preview renders — **the highest-risk item**: preview writes to `%TEMP%\slop-video-downloader\…` and those backslash paths must survive `convertFileSrc` into the `<video>` tag
3. No console windows flash during fetch, preview or export
4. Reveal-in-folder selects the file in Explorer

Report these four as the outstanding checklist.

---

## Notes for the implementer

- **Task 1 must land and go green before Tasks 2–6.** If the Windows bundler needs configuration, that is a `tauri.conf.json` change outside this plan and worth knowing before writing Rust.
- **`cfg` arms cannot be tested locally on macOS.** The Windows CI job running `cargo test` is the only check on the `#[cfg(windows)]` branches. Expect to iterate there.
- **Test counts assume tasks run in order**: 46 → 47 (Task 2) → 49 (Task 3) → 50 (Task 4) → 51 (Task 5). If you run out of order, adjust expectations rather than assuming a regression.

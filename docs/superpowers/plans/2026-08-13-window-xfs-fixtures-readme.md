# Window, XFS Limit, Fixtures, and README Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the desktop window open at a reliable, work-area-centered size, improve the project README, raise the XFS materialization ceiling to 2 GiB, and provide reproducible SquashFS/XFS test fixtures.

**Architecture:** Keep the Slint UI and Windows startup positioning in `linuxfs-preview`, keep XFS safety limits inside `linuxfs-xfs`, and keep fixture creation in a standalone PowerShell/WSL helper. Generated images stay outside Git; tests continue to use bounded in-memory readers and source-integrity rules.

**Tech Stack:** Rust, Slint, Win32 `GetSystemMetrics`, `xfs-core`, SquashFS tooling, PowerShell, WSL2.

## Global Constraints

- Linux filesystem sources remain read-only at every layer.
- XFS materialization is capped at exactly 2 GiB (`2 * 1024 * 1024 * 1024` bytes).
- XFS file reads remain capped at 64 MiB because the current parser materializes file contents.
- Generated fixtures are disposable local test artifacts and must not be committed.
- The desktop app remains Windows 10/11 x64 and must continue shipping with `winfsp-x64.dll`.

---

### Task 1: Reproduce and fix startup centering

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs`

**Interfaces:**
- Consumes: Slint `Window::show`, `Window::size`, and `Window::set_position`.
- Produces: A Windows startup path that shows the native window before positioning it and centers it within the primary monitor work area.

- [ ] **Step 1: Write the failing geometry test**

Add a pure helper test proving that a 1200×820 window is centered in a 1680×1010 work area at `(240, 95)`.

- [ ] **Step 2: Run the focused test and verify the intended failure**

Run `cargo test -p linuxfs-preview centered_window_position -- --exact`. It must fail because the helper does not yet exist.

- [ ] **Step 3: Implement the minimal positioning helper**

Use `SM_XWORKAREA`, `SM_YWORKAREA`, `SM_CXWORKAREA`, and `SM_CYWORKAREA`. Call `window.show()?` before calculating and applying the position, then let `window.run()` own the event loop. Set the Slint window height to 820px.

- [ ] **Step 4: Run the focused test and build**

Run `cargo test -p linuxfs-preview centered_window_position -- --exact` and `cargo build --release -p linuxfs-preview`. Both must exit successfully.

### Task 2: Raise and regression-test the XFS image ceiling

**Files:**
- Modify: `crates/linuxfs-xfs/src/lib.rs`
- Modify: `crates/linuxfs-backends/src/lib.rs`
- Modify: `README.md`
- Modify: `docs/Dev-logs.md`

**Interfaces:**
- Consumes: `XfsReadOnlyBackend::open` and the backend registry’s bounded-reader tests.
- Produces: A 2 GiB fail-closed XFS image limit and boundary regression coverage.

- [ ] **Step 1: Write failing boundary tests**

Extract a small `validate_image_length(length)` helper and test that exactly 2 GiB is accepted by the size gate while 2 GiB + 1 returns `UnsupportedFeature`. Keep the test independent of allocation by testing the helper rather than opening a synthetic 2 GiB reader.

- [ ] **Step 2: Run the focused backend test and verify failure**

Run `cargo test -p linuxfs-backends oversized_xfs -- --nocapture`. It must fail against the current 512 MiB constant.

- [ ] **Step 3: Implement the 2 GiB constant and messages**

Replace the 512 MiB image ceiling with `2 * 1024 * 1024 * 1024`, route `open` through `validate_image_length`, and update the user-facing error to say `2 GiB`. Keep the 64 MiB file limit unchanged.

- [ ] **Step 4: Run focused backend/XFS tests**

Run `cargo test -p linuxfs-backends` and `cargo test -p linuxfs-xfs`. Both must pass.

### Task 3: Add reproducible SquashFS and XFS fixture generation

**Files:**
- Create: `tools/generate-linux-fixtures.ps1`
- Modify: `.gitignore`
- Create locally, ignored: `tests/fixtures-linux/generated/squashfs.img`
- Create locally, ignored: `tests/fixtures-linux/generated/xfs.img`

**Interfaces:**
- Consumes: WSL2 `mksquashfs` and `mkfs.xfs`/`xfsprogs`.
- Produces: A script that creates a small SquashFS image and a sparse XFS image without touching source disks.

- [ ] **Step 1: Add ignored fixture paths and script contract**

Ignore `tests/fixtures-linux/generated/` and make the script create a temporary WSL source directory, build a compressed SquashFS image, create a sparse XFS image, format it, and report the output paths. Fail with a clear prerequisite message if `mkfs.xfs` is unavailable in WSL.

- [ ] **Step 2: Run the generator**

Run `powershell -ExecutionPolicy Bypass -File .\tools\generate-linux-fixtures.ps1`. Verify both files exist and identify as SquashFS/XFS from their signatures.

- [ ] **Step 3: Validate fixtures through the CLI**

Run `cargo run -p linuxfs-cli -- inspect .\tests\fixtures-linux\generated\squashfs.img` and the equivalent XFS command. Confirm the SquashFS fixture probes successfully; if the current XFS parser rejects the minimal formatted image, preserve the image and report the parser’s exact error rather than weakening validation.

### Task 4: Rewrite the README professionally

**Files:**
- Modify: `README.md`

**Interfaces:**
- Consumes: The sister project’s product-oriented README structure and this repository’s actual implementation/documentation.
- Produces: A concise, professional README with product summary, feature table, safety promise, architecture, usage, supported filesystems/limits, prerequisites, development checks, and documentation links.

- [ ] **Step 1: Replace the current README structure**

Use descriptive headings, a compact feature table, code blocks for commands, and explicit read-only/WinFsp prerequisites. Do not claim WSL2, VHDX, QCOW2, write access, or support that this repository does not implement.

- [ ] **Step 2: Validate links and claims against the repository**

Check every referenced path exists and verify version/build/package statements against the current files and scripts.

### Task 5: Verify, document, package, and commit

**Files:**
- Modify: `docs/Dev-logs.md`

- [ ] **Step 1: Run formatting, lint, tests, and builds**

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --exclude linuxfs-preview`, `cargo build --workspace`, and `cargo build --release --workspace`.

- [ ] **Step 2: Run the UI mechanical detector**

Run `node C:\Users\MZou\.codex\skills\impeccable\scripts\detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs` and resolve any findings.

- [ ] **Step 3: Update the development log**

Record the 820px work-area centering, 2 GiB XFS ceiling, fixture generator, README restructure, and verification results.

- [ ] **Step 4: Package the Windows release**

Run `powershell -ExecutionPolicy Bypass -File .\tools\package-release.ps1 -Tag <version> -WinFspDll 'C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll'` and verify the ZIP contains exactly `LinuxFSManager.exe` and `winfsp-x64.dll`.

- [ ] **Step 5: Commit and push**

Stage only intentional tracked files, leave generated fixture images and `public/` untracked, then commit with `fix: center window and raise XFS tolerance` and push `master`.

# WinFsp Prerequisite Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Gate the Windows desktop application on a live, read-only WinFsp framework assessment and record each assessment as a local TOML diagnostic.

**Architecture:** `linuxfs-winfsp` owns the structured runtime, registry, and launcher-service assessment. `linuxfs-app` owns atomic text persistence in `%LOCALAPPDATA%`, and `linuxfs-preview` owns the prerequisite screen, official-download action, and recheck flow. The recorded TOML never authorizes mounting; every decision uses a fresh assessment.

**Tech Stack:** Rust, Slint, WinFsp Rust bindings, Windows Service Control Manager APIs, Windows registry APIs, TOML text, PowerShell release packaging.

## Global Constraints

- Linux filesystem sources remain read-only at every layer.
- The application must not download, install, start, stop, or modify WinFsp or any Windows service.
- `WinFsp.Launcher` must be installed and running, the architecture runtime DLL must exist, and `winfsp_init` must succeed before the app is ready.
- `%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml` is diagnostic only and must never bypass a live prerequisite failure.
- The portable ZIP continues to include `winfsp-x64.dll` but does not claim to contain the WinFsp driver/framework.

---

### Task 1: Add a structured, live WinFsp assessment

**Files:**
- Modify: `crates/linuxfs-winfsp/Cargo.toml`
- Modify: `crates/linuxfs-winfsp/src/lib.rs`

**Interfaces:**
- Produces: `pub struct WinFspAssessment`, `pub enum WinFspRequirement`, and `pub fn assess_winfsp() -> WinFspAssessment`.
- Consumes: registered installation directory, architecture runtime path, the `WinFsp.Launcher` SCM service, and `winfsp::winfsp_init()`.

- [ ] **Step 1: Write failing pure assessment tests**

Add tests for the readiness reducer:

```rust
assert_eq!(
    WinFspAssessment::from_checks(true, true, WinFspLauncherStatus::Running, true).requirement(),
    WinFspRequirement::Ready,
);
assert_eq!(
    WinFspAssessment::from_checks(true, true, WinFspLauncherStatus::Stopped, true).requirement(),
    WinFspRequirement::LauncherNotRunning,
);
```

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test -p linuxfs-winfsp assessment -- --nocapture`.

Expected: the new assessment symbols are unresolved.

- [ ] **Step 3: Implement the smallest live assessment**

Add the `Win32_System_Services` feature. Query `WinFsp.Launcher` with `OpenSCManagerW`, `OpenServiceW`, and `QueryServiceStatusEx`; close every acquired service handle. Check the registered path, runtime DLL path, launcher state, and initialization in that order. Return the first unmet `WinFspRequirement`; do not call initialization when an earlier requirement fails.

- [ ] **Step 4: Run focused tests**

Run: `cargo test -p linuxfs-winfsp assessment -- --nocapture`.

Expected: all reducer tests pass.

### Task 2: Record the assessment atomically without trusting it

**Files:**
- Modify: `crates/linuxfs-app/src/runtime.rs`
- Modify: `crates/linuxfs-app/src/lib.rs`

**Interfaces:**
- Consumes: `linuxfs_winfsp::WinFspAssessment`.
- Produces: `pub fn record_winfsp_assessment(assessment: &WinFspAssessment) -> Result<PathBuf>` and `pub fn winfsp_status_path() -> PathBuf`.

- [ ] **Step 1: Write failing status-record tests**

Add a test that writes an assessment to a temporary status path and asserts the resulting TOML includes `status_version = 1`, `status = "ready"`, and `launcher_service = "running"`.

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test -p linuxfs-app winfsp_status -- --nocapture`.

Expected: the record function and status path do not exist.

- [ ] **Step 3: Implement diagnostic persistence**

Format only non-sensitive assessment fields into TOML and write through `linuxfs_config::write_text_atomic`. Use `%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml` on Windows and the platform temp directory only when `LOCALAPPDATA` is unavailable. The timestamp may be a UTC Unix-seconds integer so no clock-formatting dependency is needed.

- [ ] **Step 4: Run the focused test**

Run: `cargo test -p linuxfs-app winfsp_status -- --nocapture`.

Expected: the TOML record test passes.

### Task 3: Add the prerequisite screen and recheck flow

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs`

**Interfaces:**
- Consumes: `assess_winfsp`, `record_winfsp_assessment`, and `WinFspRequirement`.
- Produces: a prerequisite screen with **Download WinFsp**, **Recheck**, and concise installation guidance.

- [ ] **Step 1: Write failing UI-state tests**

Add a pure `PrerequisiteState::from_assessment` test that asserts a missing framework results in a visible prerequisite screen with `can_continue == false`, while `Ready` allows startup.

- [ ] **Step 2: Run the focused test and verify failure**

Run: `cargo test -p linuxfs-preview prerequisite_state -- --nocapture`.

Expected: the prerequisite state type is unresolved or, in a non-elevated shell, the preview test binary is blocked only by OS error 740 after successful compilation.

- [ ] **Step 3: Implement the screen and launch branch**

Assess and record before `winfsp_init`/mount-service construction. On failure, construct a compact prerequisite window that names the failed condition, opens `https://winfsp.dev/rel/` through `rfd::MessageDialog`/Windows shell handling or the existing browser-open helper, and repeats assessment on **Recheck**. On success, continue into the existing main window unchanged.

- [ ] **Step 4: Run the preview compile and UI detector**

Run: `cargo check -p linuxfs-preview --tests` and `node .agents/skills/impeccable/scripts/detect.mjs --json --scope onboard crates/linuxfs-preview/src/main.rs`.

Expected: the Rust check succeeds and the detector has no unexplained findings.

### Task 4: Document and package

**Files:**
- Modify: `README.md`
- Modify: `docs/packaging.md`
- Modify: `docs/Dev-logs.md`

- [ ] **Step 1: Update user-facing prerequisite documentation**

State that the app verifies the framework at startup, the portable DLL is insufficient by itself, the status record is diagnostic only, and installation is a user-confirmed official-MSI action.

- [ ] **Step 2: Run workspace validation**

Run: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --exclude linuxfs-preview`, `cargo build --workspace`, and `cargo build --release --workspace`.

- [ ] **Step 3: Package and verify**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\package-release.ps1 -Tag <version> -WinFspDll 'C:\Program Files (x86)\WinFsp\bin\winfsp-x64.dll'`.

Verify the ZIP contains exactly `LinuxFSManager.exe` and `winfsp-x64.dll`.

- [ ] **Step 4: Commit and push**

Stage intentional tracked files only, leave `public/` and `.agents/` untracked unless separately requested, commit with `feat: gate startup on WinFsp framework`, and push `master`.

# Mount Lifecycle and Browse Reliability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Keep the mounted source browsable through `L:`, preserve the Unmount action after source selection, and release the drive letter before another source can be mounted.

**Architecture:** Keep filesystem enumeration in the existing WinFsp adapter, add bounded Windows drive-letter availability/release verification to `NativeMountHost`, retain service ownership until teardown succeeds, and make the Slint source row and selected source update through one application-level state transition. Tests cover the platform-independent state/lifecycle logic; a Windows image smoke test covers actual WinFsp enumeration and release.

**Tech Stack:** Rust workspace, Slint, WinFsp 2.x bindings, `windows-sys`, Cargo tests and release packaging.

## Global Constraints

- **LinuxFS Manager V1 MUST NEVER WRITE TO THE SOURCE LINUX FILESYSTEM.**
- Open physical devices, partitions, and image files read-only; do not add source-writing APIs.
- All WinFsp mutating callbacks continue to return access denied.
- Preserve the existing Ext2/Ext3/Ext4 parser and physical scan behavior.
- Do not add a database, journal replay, repair, or speculative filesystem backend.
- Do not block the Slint UI thread; mount and unmount remain background operations.
- Do not claim completion without fresh formatting, lint, test, build, and smoke-test output.

## File Map

- Modify `crates/linuxfs-app/src/lib.rs`: add the source-state synchronization helper and retain mount ownership until service teardown succeeds.
- Modify `crates/linuxfs-preview/src/main.rs`: carry source IDs through pending operations, synchronize list/current-source state, and restore action flags on success or failure.
- Modify `crates/linuxfs-winfsp/src/native.rs`: check drive-letter availability before mounting and wait for the configured drive letter to disappear after unmount.
- Modify `crates/linuxfs-winfsp/Cargo.toml`: enable the minimal Windows storage API feature needed for logical-drive inspection.
- Modify `crates/linuxfs-winfsp/src/lib.rs`: extend lifecycle tests for failed unmount retention if the existing generic manager test does not already cover it.
- No source files or disk images are modified by the implementation.

### Task 1: Add failing source-state synchronization tests

**Files:**
- Modify: `crates/linuxfs-app/src/lib.rs` in the platform-independent model/helper section.

**Interfaces:**
- Produce `pub(crate) fn apply_source_mount_state(sources: &mut [SourceViewModel], current: &mut Option<SourceViewModel>, source_id: SourceId, status: SourceStatus, mount_point: Option<String>) -> bool`.
- The function updates the matching list row, updates `current` when its ID matches, and returns `false` when no row has the requested ID.

- [ ] **Step 1: Write the failing tests**

Add tests using two compatible `SourceViewModel` values. Assert that applying
`Mounted` with `Some("L:".to_owned())` updates the matching row and selected
source so `can_mount()` is false and `can_unmount()` is true. Add a second test
that applies `Compatible` with `None` and asserts the reverse. Add a missing-ID
test asserting the helper returns `false` and leaves both inputs unchanged.

- [ ] **Step 2: Run the focused tests and confirm the red state**

Run:

```powershell
cargo test -p linuxfs-app apply_source_mount_state
```

Expected: compilation/test failure because the helper does not yet exist.

- [ ] **Step 3: Implement the minimal helper**

Find the row by `SourceId`, assign its `status` and `mount_point`, and if the
selected source has the same ID clone the updated row into `current`. Do not
change source paths, filesystem metadata, or read-only flags.

- [ ] **Step 4: Run the focused tests green**

Run the same command and confirm all new tests pass.

### Task 2: Synchronize Slint mount state and pending actions

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs` around `PendingOperation`, source selection, mount callbacks, unmount callbacks, and timer completion handling.

**Interfaces:**
- Change pending variants to carry the immutable source ID:

```rust
PendingOperation::Mount(BackgroundOperation<String>, SourceId)
PendingOperation::Unmount(BackgroundOperation<()>, SourceId)
```

- Map completion values to `CompletedOperation::Mount(Result<(SourceId, String), String>)` and `CompletedOperation::Unmount(Result<SourceId, String>)`.
- Reuse `linuxfs_app::apply_source_mount_state` for both the source row and `current_source`.

- [ ] **Step 1: Add a regression test for capability derivation**

Extend the existing preview UI-state tests with a mounted `SourceViewModel`
selection case. The expected state is `can_mount == false` and
`can_unmount == true`; a compatible unmounted source must produce the inverse.

- [ ] **Step 2: Run the preview test to establish the current behavior**

Run:

```powershell
cargo test -p linuxfs-preview ui_state_tracks_source_and_mount_capabilities
```

Expected: the new case fails until selection derives capabilities from the
source’s actual status.

- [ ] **Step 3: Update selection and operation payloads**

When a list row is selected, set the UI capabilities from
`source.can_mount()`/`source.can_unmount()` instead of always calling the
compatible-state path. Capture `source.id` in the mount/unmount background
operation and store it in the pending variant. When an operation starts, set
both mount and unmount UI flags to `false` so a second click cannot race the
same service lock.

- [ ] **Step 4: Synchronize completion and failure states**

On mount success, call `apply_source_mount_state(..., Mounted, Some(point))`,
then show only Unmount/Open Explorer. On unmount success, call it with
`Compatible, None`, then show only Mount. On mount failure, restore capabilities
from the still-compatible current source. On unmount failure, preserve the
mounted source and re-enable Unmount/Open Explorer so the user can retry; show
the returned error in the status line.

- [ ] **Step 5: Run preview tests and formatting**

Run:

```powershell
cargo fmt --all -- --check
cargo test -p linuxfs-preview
```

Expected: formatting is clean and the preview unit tests pass. The runtime test
may still require elevation on this Windows machine; record that separately if
Cargo reports Windows error 740.

### Task 3: Make drive-letter teardown observable and retryable

**Files:**
- Modify: `crates/linuxfs-winfsp/Cargo.toml`
- Modify: `crates/linuxfs-winfsp/src/native.rs`
- Modify: `crates/linuxfs-app/src/lib.rs`

**Interfaces:**
- Add the `windows-sys` `Win32_Storage_FileSystem` feature.
- Add private Windows helpers in `native.rs`:

```rust
fn drive_letter_mask(mount_point: &str) -> linuxfs_core::Result<u32>;
fn drive_letter_is_present(mask: u32, mount_point: &str) -> bool;
fn wait_for_drive_release(mount_point: &str) -> linuxfs_core::Result<()>;
```

- `wait_for_drive_release` must poll `GetLogicalDrives` for a bounded period,
  sleep between polls, and return a WinFsp failure with the drive letter in the
  message when the bit remains set.

- [ ] **Step 1: Add pure drive-letter helper tests**

Test valid `L:`/`l:` parsing, reject non-drive paths, assert the expected bit
for `L:` is detected, and assert a different drive bit does not satisfy the
check. Keep the tests independent of an actual physical drive.

- [ ] **Step 2: Run the focused WinFsp tests and confirm the new helper tests are red**

Run:

```powershell
cargo test -p linuxfs-winfsp drive_letter
```

Expected: compilation/test failure before the helpers and API feature are
implemented.

- [ ] **Step 3: Implement bounded availability/release checks**

Before starting a host, reject a configured drive letter already reported by
`GetLogicalDrives`. After `host.unmount()` and `host.stop()`, wait until the
same drive bit disappears. Set `started = false` only after the host has been
stopped; if release verification fails, return an error while preserving the
started state needed for the service to retain ownership and retry. Do not use
filesystem probing against `L:` as the wait condition because a filesystem
callback can block.

- [ ] **Step 4: Retain service ownership on failed teardown**

In `WindowsImageMountService::unmount`, obtain the manager without losing it on
failure (for example, use `get_mut` for the call and remove it only after
`manager.unmount()` returns `Ok(())`). Keep the existing source-ID validation
and one-mount-at-a-time rule. On success remove the manager from the map; on
failure leave it in the map for a retry.

- [ ] **Step 5: Run focused tests**

Run:

```powershell
cargo test -p linuxfs-winfsp
cargo test -p linuxfs-app
```

Expected: all generic mount-manager, drive-helper, and app tests pass.

### Task 4: Verify the integrated Windows behavior

**Files:**
- Modify no source files unless a focused verification exposes a regression.
- Package output: `dist/LinuxFSManager-win64-20260813/` and its ZIP, if the local packaging convention uses the dated directory.

- [ ] **Step 1: Run repository checks**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude linuxfs-preview
cargo build --workspace
```

Expected: exit code 0 for each command and no test failures. Run
`cargo test -p linuxfs-preview` separately only if the elevation manifest can
be executed; otherwise report the OS error 740 without calling it a product
failure.

- [ ] **Step 2: Run the WinFsp image smoke test**

Start:

```powershell
cargo run -p linuxfs-winfsp --example mount_image -- tests\fixtures-ext\generated\ext4.img L:
```

While it is mounted, verify `Get-ChildItem L:\` and read a known fixture file.
Send EOF to the smoke-test process, then verify `Get-PSDrive -Name L` reports
no drive. This proves enumeration, file reads, and drive-letter release.

- [ ] **Step 3: Build and package the executable**

Run:

```powershell
cargo build --release -p linuxfs-preview
```

Copy the resulting `target\release\LinuxFSManager.exe` and the reviewed
`winfsp-x64.dll` beside each other in the dated `dist` folder. Verify file
existence, executable timestamp/size, and SHA-256 hashes; create the ZIP only
from that folder. Do not claim the package is standalone: installed WinFsp is
still a prerequisite.

- [ ] **Step 4: Report exact artifact paths and limitations**

Provide the absolute executable path, package directory, ZIP path, and current
verification results. State explicitly that the physical-drive test still
requires the user’s elevated environment and that no source writes were used.

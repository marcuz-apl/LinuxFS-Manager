# Physical Scan Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Preserve and expose per-drive physical scan diagnostics, write a safe local scan log, and rebuild the Windows package.

**Architecture:** Keep raw access, GPT/MBR parsing, and Ext probing in `linuxfs-windows`; return a report with discovered sources plus bounded diagnostic lines. The application formats failures for the UI and writes only diagnostic metadata to `%LOCALAPPDATA%`.

**Tech Stack:** Rust workspace, Windows raw read APIs, Slint, WinFsp, Cargo.

## Global Constraints

- Physical sources remain read-only at every layer.
- No source filesystem contents are logged.
- No WinFsp, parser, or filesystem write APIs are added.
- Existing image integrity tests and workspace checks must continue to pass.

### Task 1: Add report formatting tests

**Files:**
- Modify: `crates/linuxfs-windows/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add a test for a report containing one diagnostic line and assert the rendered
text includes the application header, the drive diagnostic, and the discovered
partition count.

- [ ] **Step 2: Run the focused test**

Run `cargo test -p linuxfs-windows physical_scan_report` and confirm it fails
because the report type/rendering is not present.

### Task 2: Implement physical scan diagnostics

**Files:**
- Modify: `crates/linuxfs-windows/src/lib.rs`

- [ ] **Step 1: Implement the report type and renderer**

Add `PhysicalScanReport` with discovered partitions and diagnostic lines, plus
`render()` that emits bounded text.

- [ ] **Step 2: Add raw-read and partition probe diagnostics**

Record drive open/size results, first 16 bytes at offsets 0 and 512, layout
errors, partition ranges, partition-start bytes, Ext magic at relative offset
1080, and exact Ext probe results.

- [ ] **Step 3: Preserve open failures**

Make the scan loop record an error for every failed `PhysicalDiskReader::open`
instead of dropping the drive.

- [ ] **Step 4: Run focused tests**

Run `cargo test -p linuxfs-windows` and confirm all tests pass.

### Task 3: Surface and persist diagnostics in the app

**Files:**
- Modify: `crates/linuxfs-app/src/lib.rs`
- Modify: `crates/linuxfs-preview/src/main.rs`

- [ ] **Step 1: Add a report-returning Windows source scan path**

Use the new report in `WindowsSourceProvider::refresh`; return the rendered
report as the error when no compatible partition is found.

- [ ] **Step 2: Write the report to the local diagnostic path**

Add a small Windows-only helper that creates `%LOCALAPPDATA%\\LinuxFS Manager`
and atomically replaces `scan.log`; report failures must not affect scanning.

- [ ] **Step 3: Display the detailed error**

Keep the existing UI flow but show the returned report in the source details and
status fields.

- [ ] **Step 4: Run app tests**

Run `cargo test -p linuxfs-app` and `cargo test -p linuxfs-preview`.

### Task 4: Verify and package

**Files:**
- Modify: `HANDOFF.md` only if the rebuilt artifact or behavior needs recording.

- [ ] **Step 1: Run repository verification**

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace`.

- [ ] **Step 2: Build the Windows release executable**

Run `cargo build --release -p linuxfs-preview` on the available Windows toolchain.

- [ ] **Step 3: Refresh the package**

Copy the release executable and the reviewed `winfsp-x64.dll` into
`dist/LinuxFSManager`, recreate `dist/LinuxFSManager-win64.zip`, and verify both
files are present beside each other.

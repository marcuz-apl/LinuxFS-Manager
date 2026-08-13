# Light Client, Dark Caption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the 1.7.1 light client-area palette while applying a dark native Windows caption bar without changing filesystem or mount behavior.

**Architecture:** Keep the Slint layout and application callbacks intact. A Windows-only helper obtains the Slint native window handle after `show()`, then asks DWM to use immersive dark caption mode; a failed call leaves the normal native title bar intact. The only UI-model changes are colors in the existing Slint declaration.

**Tech Stack:** Rust 2024, Slint 1.13, `raw-window-handle` 0.6, `windows-sys` 0.61 DWM bindings, PowerShell release packaging.

## Global Constraints

- Preserve all 1.7.6 mount lifecycle, source identity, unmount recovery, and dynamic drive-letter behavior exactly.
- Linux source filesystems remain read-only at every layer.
- Keep native Windows drag, minimize, maximize, close, resize, and accessibility behavior; do not create custom window chrome.
- Use the 1.7.1 pale-blue source/status surfaces and retain blue only for primary actions and selected-source emphasis.
- Do not push this branch.

---

### Task 1: Add the native dark-caption boundary

**Files:**
- Modify: `crates/linuxfs-preview/Cargo.toml`
- Modify: `crates/linuxfs-preview/src/lib.rs:1-22`
- Modify: `crates/linuxfs-preview/src/main.rs:364-412, 525-526`

**Interfaces:**
- Consumes: `&slint::Window` after `window.show()?`.
- Produces: `#[cfg(windows)] fn enable_dark_caption(window: &slint::Window)`; it has no return value and never changes app state if native DWM support is unavailable.

- [x] **Step 1: Write the failing Windows-only unit test for the DWM attribute selector**

```rust
#[cfg(windows)]
#[test]
fn dark_caption_attribute_is_immersive_dark_mode() {
    assert_eq!(dark_caption_attribute(), 20);
}
```

Define `dark_caption_attribute() -> u32` in `crates/linuxfs-preview/src/lib.rs` under `#[cfg(windows)]` so the helper and its test share one documented attribute value.

- [x] **Step 2: Run the focused library test and verify it fails**

Run: `cargo test -p linuxfs-preview --lib dark_caption_attribute_is_immersive_dark_mode`

Expected: compilation fails because `dark_caption_attribute` does not exist.

- [x] **Step 3: Implement the smallest native helper**

Add the direct dependency and DWM feature:

```toml
[dependencies]
raw-window-handle = "0.6"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_Graphics_Dwm",
  "Win32_UI_Shell",
  "Win32_UI_WindowsAndMessaging",
] }
```

Use `raw_window_handle::{HasWindowHandle, RawWindowHandle}` to extract the Win32 `HWND` from `window.window_handle()`. Call `DwmSetWindowAttribute` with an `i32` value of `1`, `linuxfs_preview::dark_caption_attribute()`, and an FFI-size argument. Ignore a missing/non-Win32 handle and an error HRESULT. Put the FFI call inside the smallest `unsafe` block with a `SAFETY:` comment. Invoke the helper immediately after the main window's existing `window.show()?` and before `center_window(window.window())`.

- [x] **Step 4: Run focused checks**

```powershell
cargo fmt --all -- --check
cargo test -p linuxfs-preview --lib dark_caption_attribute_is_immersive_dark_mode
cargo check -p linuxfs-preview
```

Expected: all commands succeed; missing DWM support is a non-fatal visual fallback.

- [x] **Step 5: Commit the native caption boundary**

```powershell
git add crates/linuxfs-preview/Cargo.toml crates/linuxfs-preview/src/lib.rs crates/linuxfs-preview/src/main.rs Cargo.lock
git commit -m "feat: darken native window caption"
```

### Task 2: Restore the restrained 1.7.1 client palette

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs:30-164`
- Modify: `docs/Dev-logs.md:7-20`

**Interfaces:**
- Consumes: the existing `source_names`, `selected_source`, `can_mount`, and `can_unmount` Slint properties and callbacks.
- Produces: the same component and callbacks with only visual color/surface changes.

- [x] **Step 1: Establish the pre-change build baseline**

```powershell
cargo check -p linuxfs-preview
node .agents/skills/impeccable/scripts/detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs
```

Expected: the existing window compiles and the detector reports pre-change UI diagnostics.

- [x] **Step 2: Apply the light palette without changing layout or callbacks**

In the existing `MainWindow` declaration, set the source panel to `#f7fafe` with border `#e3edf7`; source rows to selected `#dbeafe`/`#124f86` and unselected `#38536d`; the detail surface to `#f9fbfd` with border `#e6edf5`; and the status panel to `#edf5fb` with border `#d8e5f0`. Restore 1.7.1 text colors: `#17324d`, `#6b7c93`, `#71849a`, `#526a83`, `#245f47`, and `#46627a`. Keep the current layout dimensions, image icon, empty state, primary **Mount** button, and every property/callback name unchanged.

- [x] **Step 3: Record the presentation change**

Add a dated `docs/Dev-logs.md` entry stating that the workspace reverted from the high-contrast rail/status treatment to the 1.7.1 light palette and now uses a native dark caption when supported. State explicitly that mount logic and read-only behavior did not change.

- [x] **Step 4: Verify the completed UI and package**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude linuxfs-preview -q
cargo build --workspace
node .agents/skills/impeccable/scripts/detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs
powershell -ExecutionPolicy Bypass -File tools/package-release.ps1
```

Expected: all repository checks pass; the detector runs once on the finished UI; the release directory contains both `LinuxFSManager.exe` and `winfsp-x64.dll`.

- [ ] **Step 5: Perform the bounded manual Windows check**

Launch the rebuilt `LinuxFSManager.exe` as Administrator, then verify the centered native dark caption, light client palette, selected-source highlight, mount/Explorer/unmount flow, and next-free-letter selection for a second mount. If the OS declines dark caption mode, verify that the normal native caption and client UI remain usable.

- [x] **Step 6: Commit the visual refinement and documentation**

```powershell
git add crates/linuxfs-preview/src/main.rs docs/Dev-logs.md VERSION
git commit -m "feat: restore light workspace palette"
```

## Plan Self-Review

- **Spec coverage:** Task 1 supplies the native dark caption with a safe fallback; Task 2 restores the 1.7.1 light palette, retains 1.7.6 behavior, documents the change, and validates the package.
- **No placeholders:** The plan names the only files, dependency features, helper interface, palette values, checks, and manual acceptance criteria.
- **Type consistency:** `enable_dark_caption` consumes only `&slint::Window`; `dark_caption_attribute() -> u32` is defined in the preview library and verified there.

# UI Polish and App Icon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give LinuxFS Manager a comfortable Windows-native desktop layout and embed a recognizable read-only storage icon in the Windows executable.

**Architecture:** Keep all application behavior and callbacks unchanged. Update the existing Slint surface in `crates/linuxfs-preview/src/main.rs`, add a project-local vector icon source plus generated Windows ICO, and configure `winres` in `crates/linuxfs-preview/build.rs` to embed the icon resource.

**Tech Stack:** Rust, Slint 1.13, WinRes 0.1, SVG, ICO/PNG assets, Cargo release build.

## Global Constraints

- Preserve LinuxFS Manager V1's absolute read-only source safety rule.
- Do not modify filesystem parsing, physical-device reads, WinFsp callbacks, mount lifecycle, or configuration behavior.
- Keep the existing Windows elevation manifest and GUI subsystem setting.
- Keep the product executable named `LinuxFSManager.exe`.
- Do not add a database or a runtime write capability.

---

### Task 1: Add the deterministic app icon assets

**Files:**
- Create: `assets/linuxfs-manager.svg`
- Create: `assets/linuxfs-manager.png`
- Create: `assets/linuxfs-manager.ico`

**Interfaces:**
- Produces the ICO path `assets/linuxfs-manager.ico` for `winres::WindowsResource::set_icon`.
- Produces the PNG source for inspecting the mark and future packaging use.

- [ ] **Step 1: Add the vector source**

Create a square 256x256 SVG with a solid or transparent background, a rounded blue storage-drive body, and a centered white shield/check mark. Use no text, gradients, shadows, or tiny details that disappear at 16x16.

- [ ] **Step 2: Render the PNG**

Run:

```powershell
ffmpeg -y -i assets/linuxfs-manager.svg -vf scale=256:256 assets/linuxfs-manager.png
```

Expected: a 256x256 PNG with the same crisp drive/shield mark.

- [ ] **Step 3: Create the Windows ICO**

Use the installed image conversion tool available on the machine to create an ICO containing 16, 32, 48, 64, 128, and 256 pixel representations from the PNG. Verify the output exists and is non-empty.

- [ ] **Step 4: Inspect the generated assets**

Open the PNG and verify the mark is centered, legible, blue/white, and has no accidental text or fringe. Confirm the ICO file is present before wiring it into the build.

- [ ] **Step 5: Commit the asset-only change**

```powershell
git add assets/linuxfs-manager.svg assets/linuxfs-manager.png assets/linuxfs-manager.ico
git commit -m "feat: add LinuxFS Manager app icon"
```

### Task 2: Embed the icon in the Windows resource build

**Files:**
- Modify: `crates/linuxfs-preview/build.rs`

**Interfaces:**
- Consumes `assets/linuxfs-manager.ico`.
- Produces an executable resource containing the application icon while retaining the current elevation manifest.

- [ ] **Step 1: Add the icon resource call**

In the existing Windows-only `WindowsResource` setup, call:

```rust
resource.set_icon("../../assets/linuxfs-manager.ico");
```

Keep the existing manifest and `compile()` call intact.

- [ ] **Step 2: Build the preview package**

Run:

```powershell
cargo build --release -p linuxfs-preview
```

Expected: the build succeeds and produces `target/release/LinuxFSManager.exe`.

- [ ] **Step 3: Commit the resource wiring**

```powershell
git add crates/linuxfs-preview/build.rs
git commit -m "feat: embed LinuxFS Manager icon"
```

### Task 3: Polish the Slint desktop layout

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs`

**Interfaces:**
- Preserve existing properties and callbacks: `source_names`, `selected_source`, `source_selected`, `mount_clicked`, `unmount_clicked`, `open_explorer_clicked`, `details_clicked`, `refresh_clicked`, `scan_drives_clicked`, and `open_image_clicked`.
- Preserve current Rust-side status and capability updates.

- [ ] **Step 1: Set comfortable window dimensions**

Set the window to approximately `width: 980px; height: 620px; min-width: 860px; min-height: 560px;` so the primary action row fits at first launch while still allowing resizing.

- [ ] **Step 2: Build the native-polished header**

Replace the plain title row with a left-aligned drive mark built from Slint rectangles/text-free shapes, title, and a muted subtitle. Keep Refresh, Scan Drives, and Open Image on the right with consistent spacing.

- [ ] **Step 3: Refine the read-only banner**

Keep the existing safety copy exactly, but use a cool blue informational treatment with a compact shield/check mark and enough padding to read as an intentional status banner.

- [ ] **Step 4: Refine the source panel**

Keep the two-column structure. Give the source list a light panel background, consistent row padding, selected-source blue highlight, and hover feedback using `TouchArea.has-hover` where supported. Improve the detail hierarchy with a title, metadata text, and a bounded details area.

- [ ] **Step 5: Make the action row stable**

Keep the four actions in a dedicated bottom `HorizontalBox`. Give each button enough width for its label, use consistent spacing, and ensure the row remains visible at the new minimum size. Do not change the enabled expressions or callback wiring.

- [ ] **Step 6: Keep status readable**

Place the current status line in a subtle footer area with adequate contrast and no truncation at the default window size.

- [ ] **Step 7: Compile-check the UI**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --release -p linuxfs-preview
```

Expected: all commands succeed without changing application behavior.

- [ ] **Step 8: Commit the UI change**

```powershell
git add crates/linuxfs-preview/src/main.rs
git commit -m "feat: polish LinuxFS Manager desktop layout"
```

### Task 4: Package and verify the finished executable

**Files:**
- Modify only generated files under `dist/`; do not commit build output unless the repository's existing packaging workflow requires it.

**Interfaces:**
- Produces `dist/LinuxFSManager-win64-polished/LinuxFSManager.exe`.
- Produces `dist/LinuxFSManager-win64-polished/winfsp-x64.dll` beside the executable.
- Produces `dist/LinuxFSManager-win64-polished.zip`.

- [ ] **Step 1: Run the workspace checks**

Run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

If the elevated preview test cannot execute from the current shell, record that exact limitation while confirming the workspace compiles and the non-preview tests pass.

- [ ] **Step 2: Verify the PE subsystem and icon resource**

Inspect `target/release/LinuxFSManager.exe` and verify it remains `Windows GUI` subsystem and has a non-empty icon resource.

- [ ] **Step 3: Create the distributable folder and ZIP**

Copy the release executable and installed `winfsp-x64.dll` into the package folder, create the ZIP, and record SHA-256 hashes for the executable, DLL, and ZIP.

- [ ] **Step 4: Report the final artifact paths**

Return clickable paths for the executable and ZIP, plus the verification results and the expected WinFsp prerequisite.

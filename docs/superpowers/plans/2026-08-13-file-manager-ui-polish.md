# File-Manager UI Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine LinuxFS Manager's main Slint window into a polished file-manager workspace without changing mount, discovery, or read-only behavior.

**Architecture:** Keep all Rust callbacks, properties, and application state untouched. Replace only the `MainWindow` Slint composition so the source model renders in a dark navigation rail and the selected source/actions render in a bright workspace. Use existing standard Slint widgets and `TouchArea` callback wiring; no dependencies or new assets are required.

**Tech Stack:** Rust, Slint 1.x declarative UI, Cargo, Impeccable layout inspection.

## Global Constraints

- The source Linux filesystem must remain read-only at every layer.
- Do not change storage, filesystem, WinFsp, mount, configuration, privilege, or prerequisite logic.
- Keep `MainWindow` property and callback names exactly as currently exposed to Rust.
- Keep the application at `1200 x 820`, centered by the existing `center_window` call.
- Keep all work local; do not push to a remote repository.

---

### Task 1: Recompose the main Slint window

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs:6-247`

**Interfaces:**
- Consumes: `source_names`, `selected_source`, `source_name`, `source_details`, `image_path`, `can_mount`, `can_unmount`, `engine_status`, and `status` properties already provided by Rust.
- Produces: The same `mount_clicked()`, `unmount_clicked()`, `open_explorer_clicked()`, `details_clicked()`, `scan_drives_clicked()`, `open_image_clicked()`, and `source_selected(int)` interactions with an updated visual presentation.

- [ ] **Step 1: Establish the visual regression baseline**

Run: `cargo check -p linuxfs-preview --tests`

Expected: PASS. The preview crate contains an elevated Windows manifest, so this compilation check is the safe baseline for its Slint UI in this environment.

- [ ] **Step 2: Replace only the `MainWindow` presentation tree**

Keep the public Slint interface unchanged and shape the updated window around this composition:

```slint
HorizontalBox {
    Rectangle { width: 316px; background: #102a43; /* source rail */ }
    VerticalBox {
        HorizontalBox { /* Scan Drives, Open Image, About */ }
        Rectangle { /* selected source, image path, actions */ }
        Rectangle { /* read-only, engine, operation status */ }
    }
}
```

Use `root.selected_source == index` for selected rail rows and retain `root.source_selected(index)` in their `TouchArea`. Wire each retained button to its existing root callback. Build the Mount control as the single blue primary control and leave the other actions as quiet secondary controls.

- [ ] **Step 3: Compile the revised UI**

Run: `cargo check -p linuxfs-preview --tests`

Expected: PASS with no Slint parser, property, callback, or Rust bridge errors.

- [ ] **Step 4: Inspect layout quality**

Run: `node .agents/skills/impeccable/scripts/detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs`

Expected: JSON report reviewed for the dark rail, bright workspace, selected/disabled states, status strip, and action hierarchy. Correct any findings that conflict with the approved design.

- [ ] **Step 5: Commit the focused UI change locally**

```powershell
git add crates/linuxfs-preview/src/main.rs
git commit -m "feat: polish file-manager workspace"
```

### Task 2: Verify the polished build

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs` only if formatting or the layout inspection in Task 1 identifies a concrete issue.

**Interfaces:**
- Consumes: The unchanged UI/Rust interface from Task 1.
- Produces: A formatted, warning-free desktop preview build whose layout is ready for user testing.

- [ ] **Step 1: Run format validation**

Run: `cargo fmt --all -- --check`

Expected: PASS. If it reports formatting changes, run `cargo fmt --all`, re-run the check, and commit only the resulting formatting adjustment.

- [ ] **Step 2: Run static analysis**

Run: `cargo clippy -p linuxfs-preview --all-targets --all-features -- -D warnings`

Expected: PASS with no warnings.

- [ ] **Step 3: Build the workspace**

Run: `cargo build --workspace`

Expected: PASS and produce the updated local executable artifacts.

- [ ] **Step 4: Review the final local diff and commit verification-only adjustments if necessary**

Run: `git diff --check; git status --short`

Expected: no whitespace errors; unrelated untracked `.agents/` and WinFsp-plan files remain untouched. If Task 2 changed tracked files, commit them with `git add <exact-file>` and `git commit -m "style: format polished workspace"`.

# Desktop Localization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Windows auto-detected and user-selectable twelve-language LinuxFS Manager UI while preserving all read-only filesystem and mount behavior.

**Architecture:** `linuxfs-config` persists only an optional BCP 47 `ui_language` override. `linuxfs-preview` owns locale resolution, Windows locale discovery, bundled message catalogs, translated dynamic-message formatting, and Slint UI properties. The storage, filesystem, WinFsp, drive-letter, and mount lifecycle crates do not receive localization dependencies or behavior changes.

**Tech Stack:** Rust 2024, Slint 1.13, serde/TOML, `windows-sys` 0.61 `Win32_Globalization`, Windows BCP 47 locale names, PowerShell packaging.

## Global Constraints

- Ship only English (`en`), French (`fr-FR`), German (`de-DE`), Spanish (`es-ES`), Portuguese (Brazil) (`pt-BR`), Italian (`it-IT`), Polish (`pl-PL`), Russian (`ru-RU`), Simplified Chinese (`zh-CN`), Traditional Chinese (`zh-TW`), Japanese (`ja-JP`), and Korean (`ko-KR`).
- Do not ship Arabic or any right-to-left locale.
- English is the fallback for missing, malformed, or unsupported locale preferences and Windows locale values.
- The language picker lives in the main-window header and lists **Automatic (Windows)** plus self-named language labels: English, Français, Deutsch, Español, Português (Brasil), Italiano, Polski, Русский, 简体中文, 繁體中文, 日本語, 한국어.
- Locale switching must not scan, reload, mount, unmount, change drive letters, or alter a Linux source filesystem.
- Filesystem names, labels, paths, UUIDs, drive letters, and raw external error details remain exact, unlocalized values.
- Do not push this work until the user asks.

---

### Task 1: Persist an optional language override safely

**Files:**
- Modify: `crates/linuxfs-config/src/lib.rs:10-47, 154-199`
- Test: `crates/linuxfs-config/src/lib.rs:154-199`

**Interfaces:**
- Produces: `AppConfig::ui_language: Option<String>`.
- Consumes: a BCP 47 locale override from the UI layer; `None` means **Automatic (Windows)**.

- [ ] **Step 1: Write the failing configuration round-trip test**

```rust
#[test]
fn config_round_trips_an_optional_ui_language() {
    let path = temp_path();
    let store = ConfigStore::new(&path);
    let config = AppConfig {
        ui_language: Some("ja-JP".into()),
        ..Default::default()
    };

    store.save(&config).expect("save");
    assert_eq!(store.load().expect("load").ui_language, Some("ja-JP".into()));
    let _ = fs::remove_file(path);
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test -p linuxfs-config config_round_trips_an_optional_ui_language`

Expected: compilation fails because `AppConfig` has no `ui_language` field.

- [ ] **Step 3: Implement the backward-compatible field**

Add this exact field beside `preferred_drive_letter`:

```rust
#[serde(default)]
pub ui_language: Option<String>,
```

Set `ui_language: None` in `Default`. Do not raise `CURRENT_CONFIG_VERSION`:
Serde’s default keeps existing version-1 config files valid, and the new field
is additive.

- [ ] **Step 4: Verify configuration behavior**

Run:

```powershell
cargo test -p linuxfs-config
cargo fmt --all -- --check
```

Expected: the new round-trip test and the existing malformed-version/default
tests pass.

- [ ] **Step 5: Commit the configuration boundary**

```powershell
git add crates/linuxfs-config/src/lib.rs
git commit -m "feat: persist UI language preference"
```

### Task 2: Add typed locale resolution and bundled catalog validation

**Files:**
- Create: `crates/linuxfs-preview/src/localization.rs`
- Modify: `crates/linuxfs-preview/src/lib.rs:1-22`
- Modify: `crates/linuxfs-preview/Cargo.toml:20-25`
- Test: `crates/linuxfs-preview/src/localization.rs`

**Interfaces:**
- Produces: `pub enum UiLanguage`, `pub const AUTOMATIC_LANGUAGE: &str = "auto"`, `pub fn resolve_language(preference: Option<&str>, windows_locale: &str) -> UiLanguage`, `pub fn windows_user_locale() -> String`, and `pub fn catalog(language: UiLanguage) -> UiCopy`.
- Consumes: `AppConfig::ui_language` and a Windows BCP 47 locale string.
- `UiCopy` contains every owned static label plus typed formatter methods for dynamic status/error messages.

- [ ] **Step 1: Write failing pure locale tests**

```rust
#[test]
fn resolver_prefers_a_supported_saved_override() {
    assert_eq!(resolve_language(Some("ko-KR"), "fr-FR"), UiLanguage::Korean);
}

#[test]
fn resolver_matches_windows_base_language_then_falls_back_to_english() {
    assert_eq!(resolve_language(None, "de-AT"), UiLanguage::German);
    assert_eq!(resolve_language(None, "nl-NL"), UiLanguage::English);
}

#[test]
fn every_catalog_contains_all_message_keys() {
    for language in UiLanguage::ALL {
        assert!(catalog(language).is_complete());
    }
}
```

- [ ] **Step 2: Compile the failing preview tests**

Run: `cargo check -p linuxfs-preview --tests`

Expected: compilation fails because `UiLanguage`, `resolve_language`, and
`catalog` do not exist. Do not use `cargo test -p linuxfs-preview` from a
non-elevated shell: the repository manifest causes Windows OS error 740 after
successful compilation.

- [ ] **Step 3: Implement the locale domain**

Create `localization.rs` with these exact `UiLanguage` variants:

```rust
English, French, German, Spanish, PortugueseBrazil, Italian,
Polish, Russian, ChineseSimplified, ChineseTraditional, Japanese, Korean
```

Give each variant its BCP 47 tag and self-name. `resolve_language` accepts a
case-insensitive saved tag first, then exact Windows tags, then these base
language mappings: `fr`, `de`, `es`, `pt`, `it`, `pl`, `ru`, `zh-Hans`,
`zh-CN`, `zh-Hant`, `zh-TW`, `ja`, and `ko`. `auto`, empty, unknown, and
malformed preference values use the supplied Windows locale; unknown Windows
locales use English.

Implement `windows_user_locale()` on Windows with
`GetUserDefaultLocaleName` and a fixed UTF-16 buffer sized to
`LOCALE_NAME_MAX_LENGTH`; return an empty string on a failed API call. On
non-Windows targets return an empty string. Add `Win32_Globalization` to the
existing `windows-sys` feature list.

Define a `UiCopy` struct for all owned static texts already present in
`MainWindow`, `about_popup`, and `PrerequisiteWindow`: headings, subtitles,
buttons, source/empty-state labels, read-only warning, About text, prerequisite
steps, and selector labels. Define typed methods for dynamic copy, including
`ready`, `refresh_failed`, `mounting`, `mounted(point)`, `unmounting`,
`unmounted`, `mount_failed(error)`, `unmount_failed(error)`,
`explorer_opened(point)`, and `existing_mount_available(error)`. Each of the
twelve languages supplies every static field and formatter template; the
catalog’s completeness test must reject a missing/empty entry.

- [ ] **Step 4: Verify the locale module**

Run:

```powershell
cargo fmt --all -- --check
cargo check -p linuxfs-preview --tests
```

Then, from an Administrator PowerShell session, run:

```powershell
cargo test -p linuxfs-preview --lib localization
```

Expected: the pure resolver and catalog-completeness tests pass; normal
non-elevated compilation remains successful.

- [ ] **Step 5: Commit the locale domain**

```powershell
git add crates/linuxfs-preview/Cargo.toml crates/linuxfs-preview/src/lib.rs crates/linuxfs-preview/src/localization.rs Cargo.lock
git commit -m "feat: add bundled UI language catalogs"
```

### Task 3: Bind localized copy and the header selector to Slint

**Files:**
- Modify: `crates/linuxfs-preview/src/main.rs:3-337, 524-568, 682-1262`
- Test: `crates/linuxfs-preview/src/main.rs:1265-1320`

**Interfaces:**
- Consumes: `UiLanguage`, `UiCopy`, `resolve_language`, `windows_user_locale`, `config_store`, and `AppConfig::ui_language`.
- Produces: `fn apply_localized_copy(window: &MainWindow, copy: &UiCopy)`, a main-header `ComboBox`, and `language_selected(int)` callback.

- [ ] **Step 1: Write failing pure UI-copy tests**

```rust
#[test]
fn localized_copy_keeps_the_read_only_guarantee_prominent() {
    let copy = localization::catalog(UiLanguage::Japanese);
    assert!(!copy.read_only_warning.is_empty());
    assert!(!copy.mounted("Z:").is_empty());
}

#[test]
fn automatic_selector_resolves_windows_language_without_mutating_mount_state() {
    let language = resolve_language(None, "zh-TW");
    assert_eq!(language, UiLanguage::ChineseTraditional);
}
```

- [ ] **Step 2: Compile the failing tests**

Run: `cargo check -p linuxfs-preview --tests`

Expected: compilation fails until the localization module and `UiCopy` API are
connected to the preview crate.

- [ ] **Step 3: Replace all owned Slint literals with localized properties**

Import `ComboBox` from `std-widgets.slint`. Add string properties for each
`UiCopy` static field and an `[string] language_options` plus
`int selected_language_index`. In the header, insert:

```slint
ComboBox {
    width: 196px;
    model: root.language_options;
    current-index: root.selected_language_index;
    selected(value) => { root.language_selected(value); }
}
```

Bind every LinuxFS Manager-owned `text:` and `placeholder-text:` in the main,
About, and prerequisite windows to localized properties. Leave dynamic source
name, filesystem description, image path, drive letter, and external error
details untouched except when passed as interpolation values to `UiCopy`
formatter methods. Keep current 1200×820 dimensions, control widths, color
palette, dark native caption helper, callbacks, and mount-state properties.

Implement `apply_localized_copy` as the sole place that sets the static
properties. Call it at startup after resolving the config override and inside
the language-selection callback. The selection callback must only update copy,
selected language index, and `AppConfig::ui_language`; it must save through
`config_store().save(&config)` and report a localized configuration-save error
if persistence fails. It must not change `state`, `current_source`,
`sources_for_ui`, or `pending`.

Pass the resolved language/copy into `run_prerequisite_gate` so the WinFsp
prerequisite screen is localized before the main window can exist. Explicit
language changes occur only in the main window; a changed selection is applied
on the next launch if WinFsp is absent.

- [ ] **Step 4: Verify UI compilation and visual structure**

Run:

```powershell
cargo fmt --all -- --check
cargo check -p linuxfs-preview --tests
node .agents/skills/impeccable/scripts/detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs
```

Expected: Slint compiles, selector fits beside existing header actions at
1200px width, and the detector has no unresolved findings.

- [ ] **Step 5: Execute elevated manual acceptance checks**

Launch the app as Administrator and verify:

1. **Automatic (Windows)** chooses a matching UI language, otherwise English.
2. Choosing French, Russian, Simplified Chinese, Traditional Chinese, Japanese,
   and Korean updates the header, buttons, About dialog, status strip, and
   WinFsp prerequisite copy immediately.
3. Close/relaunch retains an explicit language preference.
4. Selecting **Automatic (Windows)** clears the override for the next launch.
5. Changing language while a source is mounted leaves its source row, mount
   point, Explorer access, and Unmount action available.
6. No right-to-left layout or Arabic entry is present.

- [ ] **Step 6: Commit the localized UI bridge**

```powershell
git add crates/linuxfs-preview/src/main.rs
git commit -m "feat: localize desktop user interface"
```

### Task 4: Document and package v1.8.0

**Files:**
- Modify: `README.md`
- Modify: `docs/Dev-logs.md`
- Modify: `PRD.md`
- Modify: `AGENTS.md`
- Test: release package contents

**Interfaces:**
- Consumes: the twelve-language catalog and persisted `ui_language` preference.
- Produces: accurate support documentation and a portable package that embeds all translation content in `LinuxFSManager.exe`.

- [ ] **Step 1: Update user-facing documentation**

Add a README language-support section listing all twelve UI languages, the
header selector, **Automatic (Windows)** behavior, English fallback, and the
fact that filesystem names/labels are not translated. Add a Dev-log entry with
the same release facts. Update PRD configuration and UI requirements to state
the optional language preference and twelve left-to-right locale scope. Update
AGENTS only to preserve the boundary that localization is UI/app copy and may
not alter source-filesystem semantics.

- [ ] **Step 2: Run repository verification**

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude linuxfs-preview -q
cargo check -p linuxfs-preview --tests
cargo build --workspace
node .agents/skills/impeccable/scripts/detect.mjs --json --scope layout crates/linuxfs-preview/src/main.rs
```

Expected: all non-elevated checks pass. In an elevated shell, also run
`cargo test --workspace` to execute preview-library tests otherwise blocked by
the Administrator manifest.

- [ ] **Step 3: Build and inspect the portable release**

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-release.ps1
```

Verify `dist\LinuxFSManager-win64-portable\LinuxFSManager.exe`,
`winfsp-x64.dll`, `LICENSE`, and `NOTICE.md` exist, and that the ZIP contains
the same four files. Translation catalogs are compiled into the executable;
do not ship mutable external translation files.

- [ ] **Step 4: Commit documentation and release package version**

```powershell
git add README.md docs/Dev-logs.md PRD.md AGENTS.md VERSION
git commit -m "docs: document multilingual desktop support"
```

## Plan Self-Review

- **Spec coverage:** Tasks 1–3 implement the persisted override, Windows auto-detection, header selector, twelve bundled locale catalogs, translated dynamic copy, and no-RTL rule. Task 4 documents and packages the result.
- **Safety coverage:** Every task keeps translation in config/UI code; it does not modify readers, filesystems, WinFsp callbacks, mounts, drive letters, or source media.
- **Testing coverage:** The plan specifies red/green config and locale tests, a catalog completeness test, compile checks for the elevation-constrained preview target, manual mounted-source continuity validation, and package inspection.
- **No placeholders:** Files, interfaces, locale tags, selector labels, all localization boundaries, commands, and acceptance criteria are explicit.

# LinuxFS Manager development log

## Implementation history summary

- **Read-only core:** Established the Rust workspace, typed errors, bounded `BlockReader`, raw-image reads, GPT/MBR discovery, and source-integrity tests.
- **Ext support:** Added the read-only Ext2/3/4 backend, metadata/path access, image probing, CLI inspection, and image-based validation.
- **Physical discovery:** Added Windows physical-disk and volume probing, read-only handles, structured scan diagnostics, and `%LOCALAPPDATA%` scan logging to diagnose the RAID/sector-size investigation.
- **Mount reliability:** Connected the generic filesystem interface to WinFsp, fixed browse/unmount lifecycle ownership, preserved read-only denial behavior, and verified dynamic drive-letter selection.
- **Desktop polish:** Added the app icon, larger default window, About dialog, version propagation from `VERSION`, and the polished source-selection UI.
- **Current extension:** Routed all consumers through a backend registry and added SquashFS plus bounded XFS image support without adding source writes.

## 2026-08-13 — desktop localization

LinuxFS Manager now ships twelve bundled left-to-right desktop languages:
English, French, German, Spanish, Portuguese (Brazil), Italian, Polish,
Russian, Simplified Chinese, Traditional Chinese, Japanese, and Korean. The
header selector supports **Automatic (Windows)** on first launch, falling back
to English for unsupported Windows locales. Explicit choices persist in the
versioned TOML configuration without changing mount, scan, source, or drive
letter state.

Each language is also shipped as one UTF-8 TOML file in `locales\`, loaded at
startup and when the selector changes. A missing, malformed, or mismatched
file falls back to the embedded catalog. Filesystem labels, paths, UUIDs, drive
letters, and raw external errors deliberately remain unchanged. Arabic is
excluded from this release because right-to-left layout support has not been
designed or validated.

The portable package also carries four region-specific Noto Sans CJK subset
fonts: Simplified Chinese, Traditional Chinese, Japanese, and Korean. Together
they add about 22 MiB instead of roughly 60–80 MiB for full CJK families. The
application registers them privately at runtime, including in the WinFsp
prerequisite screen; they do not install or modify system fonts. Their SIL Open
Font License 1.1 text is included in the `fonts\` folder.

## 2026-08-13 — light workspace palette and dark native caption

The high-contrast navy source rail and status strip were replaced with the
calmer light-blue surfaces used in 1.7.1. The current 1200×820 workspace,
product icon, source-selection affordance, and primary Mount action remain in
place. On supported Windows versions, the app now requests a dark native title
bar with Windows-provided white caption text and controls; native drag, resize,
minimize, maximize, and close behavior are unchanged.

This is presentation-only work. The 1.7.6 mount lifecycle safeguards,
source-retention behavior, read-only filesystem boundary, and dynamic free
drive-letter selection are unchanged.

## 2026-08-13 — file-manager workspace polish

The main Slint window now uses a dark navy **Sources** rail for detected partitions and image files, plus a bright workspace for the selected source, filesystem details, image path, and mount commands. The selected source state remains backed by the existing source model and callback; scanning, opening images, mounting, unmounting, Explorer access, and details behavior are unchanged.

**Mount** is now the only primary action. **Unmount**, **Open in Explorer**, and **Details** retain their existing enabled-state behavior as secondary controls. The read-only guarantee, WinFsp engine assessment, and current operation result remain continuously visible together in a compact bottom status strip.

Validation for this presentation-only update included Slint compilation, the Impeccable layout inspection, workspace formatting and Clippy checks, the non-preview workspace test suite, a workspace build, and an optimized portable package with `winfsp-x64.dll` included.

## 2026-08-13 — mount-state continuity and drive-letter fallback

Opening an image or refreshing physical discovery no longer discards a source that the application still owns as mounted. Fresh discovery rows inherit the matching mounted state, while a still-mounted source absent from a new result remains visible so its **Unmount** control stays available.

The mount service now permits multiple concurrently mounted read-only sources. When Windows reports the preferred letter as occupied, the existing free-letter selector chooses the highest free letter instead. This applies equally when another LinuxFS Manager mount owns the original preferred drive letter.

Mount and unmount lifecycle work is serialized until each background operation has updated the source row, preventing an owned mount from being orphaned by rapid source changes. Duplicate views of the same image/partition are rejected, and a physical refresh retains its original mounted row instead of transferring ownership to a fresh scan result.

If an image probe fails, any existing mount remains in the source list and is selected for unmount. Physical-disk rescans retain the original mounted row until teardown completes instead of assigning its mount ownership to a newly scanned physical-device row.

## 2026-08-13 — push and filesystem backend extension

### Starting point

- Pushed the current application state to `origin/master` at commit `c2b5a7f`.
- The pushed version is `1.6.1`, including the dynamic free-drive-letter selection.
- The source safety contract remains read-only at the block, filesystem, WinFsp, and UI layers.

### Implementation plan

1. Preserve the existing `ReadOnlyFilesystem` interface and WinFsp adapter.
2. Add a backend registry that probes filesystem signatures and returns a common read-only backend.
3. Add SquashFS using bounded random reads over the existing `BlockReader` abstraction.
4. Add XFS using the maintained pure-Rust reader, with hard 512 MiB image and 64 MiB file materialization ceilings because that reader currently requires whole buffers.
5. Route image opening, physical scanning, CLI inspection, and mounting through the registry.
6. Add focused tests, run the workspace checks, and push the result.

### Walkthrough

The registry checks the source magic first: `hsqs` selects SquashFS, `XFSB` selects XFS, and other sources continue through the existing Ext2/3/4 probe. SquashFS keeps the source as a random-access reader and does not load the physical source wholesale. XFS is deliberately fail-closed above 512 MiB; this prevents a large physical partition from becoming an unsafe allocation.

All three backends implement only metadata, directory enumeration, file reads, and symlink reads. The WinFsp mutation callbacks remain denied, and no source write API is introduced.

### Validation notes

- SquashFS reader API: [`squashfs_reader`](https://docs.rs/squashfs_reader/latest/squashfs_reader/)
- XFS reader API and its whole-image limitation: [`xfs-core`](https://docs.rs/xfs-core/latest/xfs/)
- XFS on-disk format reference: [Linux XFS format header](https://github.com/torvalds/linux/blob/master/fs/xfs/libxfs/xfs_format.h)

The implementation was committed and pushed as `373694c` (`1.6.2 build 2026-08-13-1001`).

### Verification result

- `cargo fmt --all` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace --exclude linuxfs-preview` — passed.
- `cargo build --workspace` — passed.
- `cargo test --workspace` — all tests before the preview binary passed; the Windows preview executable was blocked by the embedded `requireAdministrator` manifest when launched from this non-elevated shell (OS error 740).
- `cargo build --release --workspace` — passed; the release executable is at
  `target/release/LinuxFSManager.exe`.

## 2026-08-13 — packaging correction

The release workflow now always copies the reviewed `winfsp-x64.dll` beside the executable and into the portable ZIP. `tools/package-release.ps1` searches the registered WinFsp installation or accepts `-WinFspDll`, then verifies both output locations before completing.

## 2026-08-13 — WinFsp prerequisite gate

LinuxFS Manager now verifies the installed WinFsp framework before it creates a mount service. The live, read-only assessment requires the Windows installation registration, the matching runtime DLL, the running `WinFsp.Launcher` service, and successful runtime initialization.

When a requirement is missing, the app opens a concise prerequisite window with an official-download action and a live **Recheck** action. It does not download or install anything, and it does not start or change a Windows service. Each assessment is recorded atomically as a diagnostic-only TOML file at `%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml`; that record never authorizes a mount.

The main application now presents its read-only guarantee, live WinFsp engine state, and current operation message together in one status panel beneath the source workspace. This makes the installed engine state visible without relying on the saved diagnostic record.

## 2026-08-13 — open-source license

LinuxFS Manager is now explicitly licensed as GPL-3.0-or-later. The repository
includes the complete GPLv3 text, a project notice file, Cargo package metadata,
and release packaging that carries the license with every portable binary.

## 2026-08-13 — About text and startup window sizing

The About window now lists the complete supported set: Ext2/3/4, SquashFS, and supported XFS images. The main window now starts with explicit 1200×820 dimensions, and its Windows startup position is calculated from the monitor work area so the window opens centered without covering the taskbar.

## 2026-08-13 — startup centering, XFS tolerance, fixtures, and README

The desktop startup path now shows the native window before positioning it and centers the 1200×820 window within the primary monitor work area, including the taskbar-safe bounds. The pure centering calculation is isolated for regression coverage; execution of the elevated preview test still requires an Administrator shell because of the application manifest.

The XFS whole-image materialization ceiling is now 2 GiB instead of 512 MiB. The 64 MiB individual-file read ceiling remains unchanged because the current upstream parser still materializes file contents. Boundary tests verify that the 2 GiB limit is accepted and the next byte fails closed.

Added `tools/generate-linux-fixtures.ps1`, backed by WSL `mksquashfs` and `mkfs.xfs`, to create disposable SquashFS and XFS images under `tests/fixtures-linux/generated/`. Both generated fixtures were inspected successfully through the CLI.

Reworked `README.md` into a product-oriented structure covering capabilities, supported filesystems, safety guarantees, architecture, usage, prerequisites, packaging, fixtures, and development checks.

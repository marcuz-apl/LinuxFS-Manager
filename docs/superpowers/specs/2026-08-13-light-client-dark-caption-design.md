# Light Client, Dark Caption Design

## Goal

Restore the calm light-blue visual language used by LinuxFS Manager 1.7.1
while giving the native Windows caption bar a dark, white-text appearance.

## Scope

- Keep the current 1.7.6 layout, controls, source-selection behavior, and all
  mount lifecycle logic unchanged.
- Replace the high-contrast navy source rail and dark status bar with the
  light source panel and light status panel palette from 1.7.1.
- Retain blue as the primary action accent and the existing product icon.
- Use the Windows Desktop Window Manager dark-caption attribute for the main
  window, preserving native drag, minimize, maximize, close, resize, and
  accessibility behavior.
- If Windows does not support the caption attribute, keep the application
  usable with the operating system's normal native caption appearance.

## Visual System

The workspace remains white against a pale blue application background. The
source list uses a nearly-white blue surface with a soft border; selected
sources receive the existing pale-blue highlight and dark-blue text. The
details card and read-only/WinFsp status panel use restrained blue tints.
Dark navy is reserved for the native caption bar and high-emphasis text,
avoiding a full-height dark panel beside the working area.

## Technical Boundaries

Only `crates/linuxfs-preview/src/main.rs` and the preview crate's Windows API
feature configuration may change. The dark caption call is isolated in a
Windows-only helper and runs after the Slint window exists. No storage,
filesystem, WinFsp, drive-letter selection, source identity, or mount-state
code changes.

## Verification

- Build and test the Rust workspace with the repository baseline checks.
- Run the Impeccable detector over the changed Slint/Rust UI source.
- Build the release package and confirm it still contains the WinFsp runtime
  DLL.
- Manually verify that the main window still centers and that a mounted source
  remains selectable and unmountable after the visual change.

# LinuxFS Manager UI Polish and App Icon Design

## Goal

Polish the existing Windows desktop surface without changing filesystem,
mounting, scanning, or read-only behavior. The application should open at a
comfortable size, keep its primary actions visible, and look intentionally
Windows-native rather than like an unstyled prototype.

## Chosen direction

Windows-native polished: restrained blue accents, white and cool-gray surfaces,
clear hierarchy, familiar controls, and no decorative effects that compete
with storage-source information.

## UI changes

- Set the default window to approximately 980 x 620, while retaining a useful
  minimum size for narrower displays.
- Use a branded header with a compact blue storage-drive mark, the title,
  subtitle, and the existing refresh, scan, and open-image actions.
- Keep the read-only promise prominent in a calm informational banner.
- Present the loaded-source area as a clear two-column layout: source list on
  the left and selected-source details on the right.
- Give source rows a strong but restrained selected state, with hover feedback
  where supported by Slint.
- Place Mount, Unmount, Open in Explorer, and Details in a dedicated bottom
  action row. Buttons use consistent sizing and remain visible at the default
  window size.
- Preserve all existing callbacks, status messages, disabled states, and
  source-selection behavior.
- Keep the interface read-only in both copy and interaction: no source-side
  write controls are introduced.

## Icon asset

Create a simple vector-style application mark: a blue storage drive with a
white read-only shield/check symbol. Produce a project-local PNG source and a
Windows `.ico` containing common sizes, then wire the icon into the Windows
resource build so the executable and taskbar use it. The icon must remain
legible at small sizes and contain no text.

## Implementation boundaries

- Modify only the Slint preview UI, Windows resource/icon packaging, and any
  small asset/build helper needed to embed the icon.
- Do not alter filesystem parsing, physical-device reads, WinFsp callbacks,
  mount lifecycle, or configuration behavior.
- Keep the existing Windows elevation manifest and GUI subsystem setting.

## Verification

- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Run `cargo test --workspace`; if the elevated preview test cannot execute,
  report that limitation while preserving its compile result.
- Build the release executable and verify the Windows PE subsystem remains
  `Windows GUI`.
- Verify the packaged executable contains the icon resource and that the
  required `winfsp-x64.dll` remains beside it.

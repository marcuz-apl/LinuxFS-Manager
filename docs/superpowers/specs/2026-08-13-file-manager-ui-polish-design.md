# File-Manager UI Polish Design

**Goal:** Give LinuxFS Manager a more confident, Windows-native file-manager appearance while preserving every discovery, mount, and read-only behavior.

## Chosen Direction

Use a bright workspace with a dark navy source rail. The rail gives physical partitions and image sources a stable home; the workspace keeps the selected source, image path, and mount controls easy to scan. Blue is reserved for the primary mount action.

## Scope

### Main window

- Keep the current `1200 x 820` window size, centered by the existing application behavior.
- Replace the light source-list card with a dark navy left rail headed **Sources** and a short supporting description.
- Preserve the existing source model, source-selection callback, selected-row state, and every source label.
- Simplify the top command bar to **Scan Drives**, **Open Image**, and **About**. The existing refresh callback remains available to Rust code but is not shown as a top-level command.
- Use a spacious white content workspace for the selected source name, filesystem details, image-path input, and actions.
- Make **Mount** the sole visually primary action. Keep **Unmount**, **Open in Explorer**, and **Details** visually quiet and retain their current enabled/disabled logic and callbacks.
- Convert the lower read-only and WinFsp messages into one compact status strip. It must always communicate read-only source protection, WinFsp engine state, and the current operation result.
- Retain the existing About dialog, prerequisite gate, title, icon, source behavior, and all Rust-to-Slint property/callback names.

## Visual Language

- Background: cool light gray-blue workspace.
- Navigation: deep navy with high-contrast white and blue-tinted secondary text.
- Action color: restrained Windows blue, used only for Mount and selected source state.
- Surfaces: clear hierarchy through spacing and subtle borders; no gradients, oversized marketing cards, or decorative effects.
- Typography: use the platform UI font with clear title, section, body, and metadata sizes.
- States: preserve readable selected, hover, disabled, and empty-list states; no functional behavior changes.

## Safety and Product Constraints

- This is presentation-only work. No storage, filesystem, WinFsp, mount, configuration, privilege, or prerequisite behavior changes.
- The read-only promise remains prominent and unambiguous.
- The UI continues to support Ext2/Ext3/Ext4, SquashFS, and bounded XFS image sources exactly as currently implemented.

## Verification

- Run `cargo fmt --all -- --check`.
- Run `cargo check -p linuxfs-preview --tests` because the elevated Windows manifest prevents directly executing that test binary in this environment.
- Run `cargo clippy -p linuxfs-preview --all-targets --all-features -- -D warnings`.
- Build the workspace with `cargo build --workspace`.
- Run Impeccable's layout detector against the Slint source and review the changed layout for contrast, spacing, state clarity, and text overflow.

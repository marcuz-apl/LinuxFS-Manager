# LinuxFS Manager

LinuxFS Manager is a Windows application for safely reading Linux Ext2, Ext3, Ext4, SquashFS, and supported XFS images from physical disks, partitions, and raw disk images.

## Current status

The repository provides a tested, read-only Rust core, a Slint/WinFsp desktop path, and a small inspection CLI:

- Exact positional reads from raw image files through read-only handles.
- Direct-image, MBR, extended/logical MBR, and GPT layout discovery.
- Checked offsets, bounded allocations, cycle-safe EBR traversal, and GPT CRC validation.
- Source-integrity regression tests that compare image bytes before and after inspection.
- Structured errors for malformed or unsupported partition metadata.
- Read-only filesystem metadata and partition-backed image mounting.
- Ext, SquashFS, and XFS read-only directory enumeration, file streaming, and symlink reads where the backend can safely provide them.
- A Slint desktop shell connected to Windows source discovery and the read-only mount service.
- `linuxfs inspect`, `linuxfs ls`, and `linuxfs cat` for scripted image access.

Physical-device access and WinFsp mounting require Windows and the WinFsp runtime. The installer/release pipeline remains separate from the tested core; see [docs/packaging.md](docs/packaging.md) for the prerequisite contract.

SquashFS is streamed through the bounded block-reader interface. The current XFS reader is deliberately limited to sources up to 512 MiB because its upstream parser requires a whole image in memory; larger XFS physical volumes are identified as unsupported rather than materialized unsafely. XFS file reads are additionally capped at 64 MiB until the parser exposes streaming extents.

Implementation decisions and validation notes are recorded in [docs/Dev-logs.md](docs/Dev-logs.md).

## V1 safety promise

LinuxFS Manager V1 must never modify the source Linux filesystem. Copying files means copying them from a mounted Linux source to a Windows destination.

See [PRD.md](PRD.md) for the full requirements and [AGENTS.md](AGENTS.md) for development and safety rules.

## CLI inspection

The CLI never opens a source for writing and streams regular-file output:

```powershell
cargo run -p linuxfs-cli -- inspect .\disk.img
cargo run -p linuxfs-cli -- ls .\disk.img /
cargo run -p linuxfs-cli -- cat .\disk.img /home/user/readme.txt
```

## UI smoke test

With a Slint-capable Rust toolchain, the standalone visual test can be launched with:

```powershell
cargo run -p linuxfs-preview
```

On Windows, it opens the supplied image read-only and connects the Mount and Unmount actions to the WinFsp adapter. On other platforms it remains a visual smoke test and does not access disk images.

This preview loads the supplied image through the Rust provider and uses the real read-only WinFsp mount service. Refresh/Open Image and the full source list remain part of the production application bridge.

# LinuxFS Manager

LinuxFS Manager is a Windows application for safely reading Linux Ext2, Ext3,
and Ext4 filesystems from physical disks, partitions, and raw disk images.

## Current status

The repository provides a tested, read-only Rust core and an early Slint/WinFsp
integration path:

- exact positional reads from raw image files through read-only handles;
- direct-image, MBR, extended/logical MBR, and GPT layout discovery;
- checked offsets, bounded allocations, cycle-safe EBR traversal, and GPT CRC validation;
- source-integrity regression tests that compare image bytes before and after inspection;
- structured errors for malformed or unsupported partition metadata;
- read-only filesystem metadata and partition-backed image mounting;
- a Slint preview connected to the read-only mount service.

The production Windows application, physical-device discovery, installer, and
final prerequisite checks remain in progress. See [docs/packaging.md](docs/packaging.md)
for the V1 packaging contract and current deployment limitations.

## V1 safety promise

LinuxFS Manager V1 must never modify the source Linux filesystem. Copying files
means copying them from a mounted Linux source to a Windows destination.

See [PRD.md](PRD.md) for the full requirements and [AGENTS.md](AGENTS.md) for
development and safety rules.

## UI smoke test

With a Slint-capable Rust toolchain, the standalone visual test can be launched with:

```powershell
cargo run -p linuxfs-preview
```

On Windows, it opens the supplied image read-only and connects the Mount and
Unmount actions to the WinFsp adapter. On other platforms it remains a visual
smoke test and does not access disk images.

This preview loads the supplied image through the Rust provider and uses the
real read-only WinFsp mount service. Refresh/Open Image and the full source list
remain part of the production application bridge.

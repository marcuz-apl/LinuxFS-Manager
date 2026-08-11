# LinuxFS Manager

LinuxFS Manager is a Windows application for safely reading Linux Ext2, Ext3,
and Ext4 filesystems from physical disks, partitions, and raw disk images.

## Milestone 1 status

The repository now provides a tested, read-only image-inspection core in Rust:

- exact positional reads from raw image files through read-only handles;
- direct-image, MBR, extended/logical MBR, and GPT layout discovery;
- checked offsets, bounded allocations, cycle-safe EBR traversal, and GPT CRC validation;
- source-integrity regression tests that compare image bytes before and after inspection;
- structured errors for malformed or unsupported partition metadata.

This milestone does not yet include Ext2/Ext3/Ext4 parsing, physical-device access,
WinFsp mounting, Slint UI, configuration, logging, or packaging. Those are later
milestones.

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

It uses mock data only and does not open, mount, or modify any disk image.

This preview is intentionally mock-only and is not the production filesystem
application or a replacement for the planned Slint bridge.

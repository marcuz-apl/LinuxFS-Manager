# LinuxFS Manager

LinuxFS Manager is a planned Windows application for safely reading Ext2, Ext3,
and Ext4 filesystems from physical disks, partitions, and raw disk images.

## V1 goals

- Read filesystem metadata and files from Windows.
- Mount supported filesystems through WinFsp in read-only mode.
- Support GPT, MBR, and raw filesystem or whole-disk images.
- Fail safely on corrupt, encrypted, or unsupported volumes.

LinuxFS Manager V1 must never modify the source Linux filesystem. Copying files
means copying them from the mounted Linux source to a Windows destination.

## Project status

This repository currently contains the product requirements and project
bootstrap. Implementation is planned in Rust with Qt/QML and WinFsp.

See [PRD.md](PRD.md) for the full requirements and [AGENTS.md](AGENTS.md) for
development and safety rules.

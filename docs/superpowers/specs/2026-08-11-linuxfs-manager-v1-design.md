# LinuxFS Manager V1 Design

**Date:** 2026-08-11  
**Status:** Approved in conversation; awaiting written-spec review  
**Scope:** Full staged V1 implementation

## Goal

Build a Windows application that reads Ext2, Ext3, and Ext4 filesystems from
raw images and physical storage without modifying the Linux source filesystem.
The implementation will be delivered in independently testable milestones so
the read-only safety boundary is proven before native Windows mounting and UI
integration are added.

## Non-negotiable constraints

- No source filesystem writes, write-enabled handles, journal replay, repair,
  formatting, partition modification, or dormant write APIs.
- All block sources expose only bounded random-access reads.
- Unknown or unsupported incompatible filesystem features fail closed.
- Image tests precede physical-device tests.
- No database is used for V1 configuration or filesystem metadata.
- QML remains a presentation layer and never parses disk structures.
- WinFsp is used as a user-mode bridge; no custom kernel driver is introduced.

## Architecture

```text
Qt/QML
  ↓
Rust application layer
  ├── discovery / commands / state / config
  └── mount manager
        ↓
WinFsp read-only adapter
        ↓
ReadOnlyFilesystem
        ↓
ExtReadOnlyBackend
        ↓
BlockReader
  ├── raw image reader
  ├── partition view
  └── Windows physical-device reader
```

The dependency direction is from application/UI consumers toward focused core
crates. Filesystem-specific types do not leak into QML or WinFsp. The first
milestone follows the existing approved image-core design in
`2026-08-11-readonly-image-core-design.md`.

## Core contracts

The public block interface is read-only:

```rust
pub trait BlockReader {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, buffer: &mut [u8]) -> Result<()>;
}
```

The generic filesystem interface contains inspection and read operations only:

```rust
pub trait ReadOnlyFilesystem {
    fn info(&self) -> Result<FilesystemInfo>;
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata>;
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>>;
    fn open_file(&self, path: &FsPath) -> Result<Box<dyn ReadOnlyFile>>;
    fn read_link(&self, path: &FsPath) -> Result<FsPath>;
}
```

Image readers and physical-device readers open sources read-only. Partition
views validate every offset and length before delegating reads. Filesystem
backends consume the bounded reader and do not care whether bytes came from an
image, partition, or physical device.

## Data flow

```text
source path/device
  → read-only BlockReader
  → GPT/MBR or direct-image detection
  → filesystem probe
  → compatibility validation
  → ExtReadOnlyBackend
  → application state
  → optional read-only WinFsp mount
```

The initial command-line surface will inspect raw images and report source
layout, filesystem identity, compatibility, and metadata. The later desktop
application will use the same Rust application services through CXX-Qt.

## Staged delivery

### Milestone 1: read-only image core

Create the Rust workspace and focused core/storage crates. Implement a raw
image reader, checked ranges, MBR parsing including bounded logical-partition
chains, GPT parsing with header and entry-array CRC validation, and direct-image
classification. Add synthetic fixtures and image hash equality tests.

### Milestone 2: filesystem registry and Ext backend

Add filesystem information types, probing, the generic read-only filesystem
interface, and an ExtReadOnlyBackend around a maintained read-only parser when
the dependency is suitable. Validate Ext2, Ext3, and Ext4 fixtures explicitly.
Reject incompatible feature flags, unsafe recovery states, corrupt metadata,
and unsupported parser capabilities with structured results.

### Milestone 3: image integration

Connect partition views and direct images to filesystem probing and file reads.
Add integration fixtures for regular files, directories, symlinks where safe,
large streamed reads, unsupported formats, corruption, and exact source-image
hash preservation after the entire read workflow.

### Milestone 4: Windows storage discovery

Add a Windows-only read-only physical-device reader and documented storage
enumeration. Keep Windows API/handle details inside the Windows storage crate.
Do not automate destructive disk preparation. Physical tests require
expendable media and run only after image tests pass.

### Milestone 5: WinFsp and mount manager

Add a Windows-only adapter translating volume information, metadata, directory
enumeration, opens, reads, cleanup, close, and safe symlink/reparse behavior.
All mutating callbacks explicitly deny create, write, truncate, delete, rename,
mkdir, timestamp, attribute, and security-descriptor changes. Add a mount
manager that owns mount lifecycle, validates mount points, prevents conflicts,
and unmounts during orderly shutdown.

### Milestone 6: Qt/QML application

Add the Rust application state/command layer and CXX-Qt bridge. The main screen
will support rescan, open image, source listing, filesystem details, prominent
`READ ONLY` status, mount/unmount, and Explorer opening. Long-running discovery,
probing, reads, and mount operations run outside the UI thread.

### Milestone 7: configuration, logging, and packaging

Add structured logging and a versioned TOML configuration at
`%APPDATA%\\LinuxFS Manager\\config.toml`. Writes are atomic, malformed config
falls back safely, and recent image paths remain bounded. Package the native
application honestly, detecting missing Qt/WinFsp prerequisites rather than
claiming a standalone binary when a driver is required.

## Error behavior

Errors preserve technical causes and use categories including
`StorageAccess`, `PermissionDenied`, `InvalidImage`, `PartitionTable`,
`UnsupportedFilesystem`, `UnsupportedFeature`, `FilesystemCorrupt`,
`FilesystemNeedsRecovery`, `MountPointUnavailable`, `WinFspUnavailable`,
`WinFspFailure`, `Configuration`, and `Internal`.

Malformed input never becomes an empty fake directory, a partial success, an
unbounded allocation, an out-of-range read, an infinite loop, or a process
panic. Application-facing code maps typed errors to concise user messages and
remediation hints without discarding the technical cause.

## Safety verification

Every storage/filesystem/mount milestone includes:

- tests written before production code and observed failing before the minimal
  implementation is added;
- checked addition/multiplication and source-range validation;
- malformed partition, superblock, directory, and path tests;
- source-image hash equality before and after inspection, probing, reading, and
  unmount workflows;
- read-only Windows operation tests for create, write, truncate, rename, delete,
  mkdir, and metadata mutation;
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
- `cargo test --workspace`;
- `cargo build --workspace`;
- native Qt/CXX-Qt and WinFsp checks when those integrations exist.

## Deliberate V1 limits

The implementation will not add XFS, Btrfs, F2FS, ZFS, LUKS, LVM, MD RAID,
indexing, cloud/network features, telemetry, repair, recovery, or source-side
copy operations. These remain future work requiring separate design approval.


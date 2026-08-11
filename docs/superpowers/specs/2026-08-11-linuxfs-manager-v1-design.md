# LinuxFS Manager V1 Design

**Date:** 2026-08-11  
**Status:** Roadmap approved in conversation; revised after design review
**Scope:** V1 roadmap; each milestone requires its own approved design and plan

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
- Shared block, filesystem, and file-handle interfaces have explicit
  thread-safety contracts before background or WinFsp consumers are added.

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

The public block interface is read-only and safe to share across worker and
filesystem callback threads:

```rust
pub trait BlockReader: Send + Sync {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, buffer: &mut [u8]) -> Result<()>;
}
```

Partition discovery receives source geometry and validates it:

```rust
pub struct BlockGeometry {
    pub logical_sector_size: u32,
}

pub fn discover_layout(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
) -> Result<SourceLayout>;
```

Raw images use a documented 512-byte logical-sector default in V1. Physical
sources use the logical sector size queried from Windows. Unsupported geometry
is rejected rather than guessed.

The generic filesystem interface contains inspection and read operations only:

```rust
pub trait ReadOnlyFilesystem: Send + Sync {
    fn info(&self) -> Result<FilesystemInfo>;
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata>;
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>>;
    fn open_file(&self, path: &FsPath) -> Result<Box<dyn ReadOnlyFile + Send + Sync>>;
    fn read_link(&self, path: &FsPath) -> Result<FsPath>;
}
```

File reads use explicit offsets and do not keep a shared mutable cursor. This
allows worker and WinFsp callback threads to perform bounded reads without
silently serializing the entire mounted filesystem.

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

Milestones 1 through 3 expose Rust library APIs and integration tests. V1 does
not add a supported command-line product; the PRD keeps the `linuxfs` companion
in V1.x. The desktop application uses the tested Rust services through CXX-Qt.

## Planning boundary

This document is the V1 roadmap, not a single implementation specification.
Only Milestone 1 currently has an approved implementation-ready design:
`2026-08-11-readonly-image-core-design.md`. Each later milestone receives a
focused design, review, implementation plan, and verification cycle after its
predecessor is complete. This keeps unresolved native integration decisions
from leaking into the image core.

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

Before implementation, record the selected parser version, maintenance state,
license, Windows support, unsupported Ext features, and adapter limitations.
The milestone design also defines reproducible fixture generation, provenance,
redistribution terms, and hashes; private or large filesystem images are not
committed.

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

The baseline privilege model keeps image workflows unelevated. Selecting a
physical source may trigger an explicit, explained UAC relaunch of the
application with the selected operation restored. V1 does not add a privileged
service or IPC helper unless a concrete WinFsp or security requirement proves
the relaunch model inadequate.

### Milestone 5: WinFsp and mount manager

Add a Windows-only adapter translating volume information, metadata, directory
enumeration, opens, reads, cleanup, close, and safe symlink/reparse behavior.
Requests carrying create, write, truncate, delete, rename, mkdir, timestamp,
attribute, or security mutation intent are denied according to the exact binding
semantics. Normal read opens remain allowed. A mount manager owns mount
lifecycle, validates mount points, prevents conflicts, and unmounts cleanly.

Before implementation, a dedicated Windows namespace mapping design must define
non-UTF-8 names, reserved Windows names, case-colliding directory entries,
separator and `..` handling, symlink containment, hard links, stable file IDs,
and representation of device nodes, sockets, and FIFOs. The WinFsp binding and
exact runtime version must be selected only after API, maintenance, license,
and write-denial behavior are reviewed.

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

Atomic replacement must use Windows replacement semantics that preserve either
the old complete file or the new complete file across failure; deleting the old
file before rename is not acceptable. Before packaging, record the selected
CXX-Qt, Qt, WinFsp, and binding versions plus their licenses and redistribution
requirements.

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


# LinuxFS Manager — Product Requirements Document

**Product:** LinuxFS Manager  
**Target platform:** Windows 10/11 x64  
**V1 scope:** Read-only access to Ext2/3/4 and SquashFS filesystems from physical disks/partitions and filesystem image files, plus bounded XFS image support
**Primary implementation language:** Rust  
**Desktop UI:** Slint (Rust-native declarative UI)  
**Windows filesystem integration:** WinFsp user-mode filesystem  
**Persistent application state:** File-based configuration; no database in V1  
**Document status:** V1 baseline

---

## 1. Product Summary

LinuxFS Manager is a small native Windows desktop application that allows users to inspect and mount Linux-native filesystems safely from Windows.

V1 focuses exclusively on **read-only Ext2/Ext3/Ext4 and SquashFS access**, with bounded XFS support for image-sized sources. Users can discover Linux partitions attached to the Windows machine or open supported raw/image files, inspect filesystem information, and mount a supported filesystem through WinFsp so that files can be browsed and copied using Windows Explorer and normal Windows applications.

The product must never write to any source filesystem in V1.

The architecture must be generic enough to add other read-only Linux filesystem backends later, such as XFS, Btrfs, and F2FS, without redesigning the application shell.

---

## 2. Goals

### 2.1 V1 goals

LinuxFS Manager V1 shall:

- Run as a normal Windows desktop application.
- Detect physical disks and partitions that may contain Linux filesystems.
- Recognize supported Ext2, Ext3, and Ext4 filesystems.
- Open raw filesystem/disk image files for testing and normal use.
- Display useful filesystem and volume metadata before mounting.
- Mount supported filesystems **read-only** through WinFsp.
- Expose mounted filesystems to Windows through a drive letter or supported WinFsp mount point.
- Allow users to browse, open, and copy files out of Linux filesystems.
- Unmount filesystems cleanly.
- Fail safely on corrupt, unsupported, encrypted, or otherwise incompatible volumes.
- Keep filesystem-specific code behind a pluggable backend interface.
- Avoid requiring a SQL database for simple application settings.

### 2.2 Product principles

1. **Read safety first.** V1 must contain no filesystem-writing feature.
2. **Fail closed.** Unknown or unsupported filesystem features must prevent mounting rather than being ignored.
3. **User mode where possible.** Do not create a custom Windows kernel filesystem driver.
4. **Portable architecture.** The GUI, block-device layer, filesystem parser, and WinFsp adapter should remain separately testable.
5. **Small V1.** Do not expand V1 into a partition editor, recovery suite, RAID manager, or disk repair tool.
6. **Test using images first.** Raw filesystem images are the preferred development and automated-testing medium before physical media is exercised.

---

## 3. Non-Goals for V1

The following are explicitly outside V1:

- Writing, modifying, deleting, renaming, chmod/chown, or creating files/directories on Linux filesystems.
- Replaying or modifying an Ext journal.
- Repairing corrupt filesystems.
- Formatting partitions.
- Creating/deleting/resizing/moving partitions.
- Converting filesystems.
- Defragmentation.
- Ext undelete/file-recovery functionality.
- LVM activation.
- LUKS decryption.
- Linux MD RAID assembly.
    - Btrfs, F2FS, ZFS, JFS, or ReiserFS mounting.
- Network filesystems.
- A background indexing/search database.
- A custom Windows kernel-mode filesystem driver.
- Automatic privilege escalation without an explicit user action.

LinuxFS Manager may **identify** unsupported/container formats where practical, but V1 shall not attempt to mount them.

---

## 4. Target Users

### 4.1 Primary users

- Windows users who dual-boot Linux and Windows.
- Developers who need read access to Linux disks.
- Engineers working with Linux appliances, embedded devices, removable media, or disk images.
- Users migrating data from Linux disks to Windows.

### 4.2 Typical use cases

1. Attach a Linux SSD to a Windows PC and copy documents from an Ext4 partition.
2. Open an `.img` or raw disk image and inspect an Ext filesystem without booting Linux.
3. Inspect filesystem UUID, label, size, and detected features.
4. Mount a filesystem read-only as `L:` and browse it in Explorer.
5. Diagnose why a particular filesystem cannot safely be mounted.

---

## 5. Key User Stories

### US-01 — Discover physical Linux filesystems

As a user, I want LinuxFS Manager to scan attached storage and show candidate Linux partitions so I can see which filesystems are available.

**Acceptance criteria:**

- The application lists physical disks and partitions accessible to it.
- Each entry includes, when available:
  - disk/partition identity
  - capacity
  - partition offset
  - partition table type
  - partition type
  - detected filesystem
  - filesystem label
  - filesystem UUID
  - mount status
- Detection itself must not write to the device.

### US-02 — Open a filesystem image

As a user, I want to open a raw image file so I can inspect or mount an Ext filesystem without using physical media.

**Acceptance criteria:**

- User can select a raw image such as `.img`, `.raw`, or an extension-less image.
- The application detects whether the image contains:
  - a directly stored Ext filesystem, or
  - a supported partition table containing an Ext filesystem.
- The image is opened read-only.
- Normal files must never be modified by LinuxFS Manager while used as filesystem sources.

### US-03 — Inspect a filesystem

As a user, I want to inspect filesystem metadata before mounting.

**Display when available:**

- filesystem family/version
- label
- UUID
- block size
- total size
- approximate used/free space when safely derivable
- feature flags
- journal presence
- filesystem state
- source type: physical device, partition, or image
- compatibility status
- reason for incompatibility

### US-04 — Mount read-only

As a user, I want to mount a supported filesystem so I can access its files through Windows Explorer.

**Acceptance criteria:**

- Mount operation is always read-only.
- The UI clearly displays `READ ONLY`.
- A free drive letter may be selected automatically or manually.
- Windows applications can enumerate directories and read files.
- Any Windows write request receives an appropriate read-only/access-denied result.
- Mounting must not replay a journal or change filesystem state.

### US-05 — Copy files to Windows

As a user, I want to copy files from the mounted Linux filesystem to an NTFS/exFAT/other Windows destination.

**Acceptance criteria:**

- Reading normal regular files works within supported Ext features.
- Large files are streamed rather than copied entirely into memory.
- Copy operation is performed by Windows/Explorer against the mounted filesystem.
- LinuxFS Manager must not alter the source.

### US-06 — Safe unmount

As a user, I want to unmount a volume cleanly.

**Acceptance criteria:**

- User can request unmount from the UI.
- Open handles are handled according to WinFsp semantics.
- UI reports success/failure clearly.
- Application shutdown attempts orderly unmount of mounts it owns.

### US-07 — Understand unsupported media

As a user, I want a useful explanation when LinuxFS Manager cannot mount something.

**Examples:**

- encrypted LUKS container
- LVM physical volume
- unsupported Ext incompatible feature
- corrupt Ext metadata
    - unsupported filesystem such as Btrfs or an XFS source above the safe image limit
- inaccessible device
- insufficient privileges

---

## 6. Functional Requirements

### FR-01 — Storage enumeration

The application shall enumerate relevant Windows storage devices using documented Windows APIs.

Implementation shall separate:

- physical disk enumeration
- partition table parsing/discovery
- raw block reading
- filesystem detection

The block layer shall expose a read-only random-access interface to filesystem backends.

Conceptual interface:

```rust
pub trait BlockReader {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, buffer: &mut [u8]) -> Result<()>;
}
```

Exact APIs may differ, but the abstraction must remain read-only in V1.

### FR-02 — Image sources

V1 shall support at minimum:

- raw filesystem images
- raw whole-disk images with MBR or GPT partitioning

Image formats requiring a complex translation layer, such as VHDX/QCOW2, are future work unless implementation is exceptionally low risk and does not delay V1.

### FR-03 — Partition tables

V1 shall detect at minimum:

- GPT
- legacy MBR

Extended/logical MBR partition support should be implemented if practical but must not compromise safety.

### FR-04 — Filesystem probing

Filesystem probing must:

- perform only bounded reads
- validate signatures and core structural metadata
- avoid interpreting arbitrary input as trusted data
- return an explicit confidence/result type
- never mount an unknown filesystem based only on a weak heuristic

### FR-05 — Read-only filesystem backends

V1 shall implement a production read-only backend registry with:

`ExtReadOnlyBackend`
`SquashfsReadOnlyBackend`
`XfsReadOnlyBackend` (bounded image sources)

Target compatibility:

- Ext2
- Ext3
- Ext4
- SquashFS 4.0
- XFS images up to the backend's documented 2 GiB safety limit

The backends may wrap maintained Rust readers behind LinuxFS Manager interfaces.
SquashFS must use bounded random reads. XFS must fail closed for sources above
the safe materialization limit until a streaming reader is available.

Dependency behavior must be validated against the application's Ext3 test corpus rather than assuming full Ext3 compatibility from an Ext2/Ext4 claim.

### FR-06 — Incompatible features

Unknown or unsupported **incompatible** Ext feature flags must cause the volume to be rejected.

The application shall present a user-readable reason where possible.

Filesystem states requiring write-side recovery must not be modified by LinuxFS Manager. If safe read-only interpretation cannot be guaranteed, reject the mount.

### FR-07 — Filesystem abstraction

Core code shall not couple the UI or WinFsp layer directly to Ext-specific types.

Conceptual interface:

```rust
pub trait ReadOnlyFilesystem {
    fn info(&self) -> Result<FilesystemInfo>;
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata>;
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>>;
    fn open_file(&self, path: &FsPath) -> Result<Box<dyn ReadOnlyFile>>;
    fn read_link(&self, path: &FsPath) -> Result<FsPath>;
}
```

Future backends should be implementable without modifying the UI architecture.

### FR-08 — WinFsp integration

Use WinFsp as the Windows filesystem bridge.

LinuxFS Manager shall implement the callbacks necessary for read-only Windows filesystem access, including as applicable:

- volume information
- security/attributes mapping
- file information
- directory enumeration
- file open
- file read
- cleanup/close
- reparse/symlink handling where safely supported

All mutating callbacks must explicitly reject writes.

Examples include:

- create
- write
- set size
- delete
- rename
- set attributes that imply source mutation
- set timestamps
- security descriptor modification

Do not rely on UI state alone to enforce read-only behavior.

### FR-09 — Mount manager

The mount manager shall:

- track mounts owned by the current application instance
- choose/validate mount points
- prevent duplicate conflicting mounts
- surface mount errors
- unmount explicitly
- clean up mounts during orderly application shutdown

### FR-10 — Slint desktop UI

Preferred V1 stack:

- Slint declarative UI and native Windows renderer
- Rust application/core logic

The UI shall remain a thin presentation layer; filesystem parsing and device access must not live in the `.slint` layer.

### FR-10a — Desktop localization

The desktop UI shall provide these left-to-right languages: English, French,
German, Spanish, Portuguese (Brazil), Italian, Polish, Russian, Simplified
Chinese, Traditional Chinese, Japanese, and Korean. On first launch it shall
use the Windows user locale when supported, otherwise English. The header shall
provide an **Automatic (Windows)** selector and an explicit-language selector.
The optional explicit preference shall be stored in the versioned configuration
file; changing language must not rescan sources, alter a mount, or change a
drive letter.

Filesystem names, labels, paths, UUIDs, drive letters, and raw external error
details remain exact values. Right-to-left languages are outside this release
until separately designed and validated.

### FR-11 — Main screen

The main screen should contain:

- Refresh/Rescan action
- Open Image action
- source list/table
- filesystem type
- size
- label
- source
- status
- mount point
- Mount button
- Unmount button
- Open in Explorer button
- Details action

Read-only status must be prominent.

### FR-12 — Privileges

Physical disk access may require elevated Windows privileges.

The product shall:

- perform non-privileged operations without elevation where possible
- request elevation only when required for the selected operation
- explain why elevated access is needed
- never hide privilege escalation from the user

The final privilege model may use a small privileged helper/service if that produces a cleaner security boundary than running the complete UI elevated.

### FR-13 — Logging

Implement structured diagnostic logging.

Logs should include:

- application version
- device discovery results
- filesystem detection result
- filesystem compatibility result
- mount/unmount events
- WinFsp integration errors
- parser errors

Logs must avoid unnecessarily storing user filenames/content.

### FR-14 — Configuration persistence

**No database is required in V1.**

Persist simple application state in a human-readable file, preferably:

`%APPDATA%\LinuxFS Manager\config.toml`

Possible settings:

- window geometry
- last selected drive letter
- recent image paths (optional and bounded)
- logging level
- update-check preference if update support is ever added
- UI preferences

Requirements:

- use versioned configuration schema
- write atomically using temporary-file + replace semantics
- recover gracefully from malformed config
- do not store filesystem contents or large metadata caches

SQLite may be introduced later only if requirements justify it, such as:

- persistent filesystem indexing
- large search catalogs
- many thousands of cached file records
- complex relational history

### FR-15 — Portable/direct execution

The desired distribution experience is a directly runnable Windows application.

The release package should minimize external setup, but WinFsp runtime/driver requirements must be handled honestly.

Acceptable V1 distribution options include:

1. installer that installs/validates WinFsp, plus the application; or
2. portable application bundle that detects WinFsp and guides installation if absent.

Do not pretend a kernel driver can simply be replaced by an arbitrary DLL.

---

## 7. Architecture

### 7.1 Logical architecture

```text
┌───────────────────────────────────────────────────────────────┐
│                       Slint UI                                │
│             (presentation; no disk parsing)                   │
└──────────────────────────┬────────────────────────────────────┘
                           │ Rust callbacks/models
┌──────────────────────────▼────────────────────────────────────┐
│                  Rust Application Layer                       │
│      discovery • commands • state • errors • config           │
└──────────────┬────────────────────────────┬───────────────────┘
               │                            │
     ┌─────────▼─────────┐        ┌─────────▼───────────┐
     │   Mount Manager   │        │ Storage Discovery   │
     │     + WinFsp      │        │  Windows APIs       │
     └─────────┬─────────┘        └─────────┬───────────┘
               │                            │
               └──────────────┬─────────────┘
                              ▼
                  ┌──────────────────────┐
                  │ Read-Only Block I/O  │
                  │ device/partition/img │
                  └──────────┬───────────┘
                             ▼
                  ┌──────────────────────┐
                  │ Filesystem Registry  │
                  │ probe + backend pick │
                  └──────────┬───────────┘
                             ▼
                  ┌──────────────────────┐
                  │ Ext Read-Only Backend│
                  │ Ext2 / Ext3 / Ext4   │
                  └──────────────────────┘
```

### 7.2 Future backend architecture

```text
ReadOnlyFilesystem
    ├── ExtReadOnlyBackend       # V1
    ├── XfsReadOnlyBackend       # future
    ├── BtrfsReadOnlyBackend     # future
    └── F2fsReadOnlyBackend      # future
```

Storage/container detection may later grow into:

```text
BlockReader
    ├── PhysicalDiskReader
    ├── PartitionReader
    ├── RawImageReader
    └── future:
        ├── LvmLogicalVolumeReader
        ├── LuksReader
        └── MdRaidReader
```

Container support is not a V1 requirement.

---

## 8. Recommended Repository Layout

```text
linuxfs-manager/
├── AGENTS.md
├── PRD.md
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── crates/
│   ├── linuxfs-core/
│   │   └── src/
│   │       ├── filesystem/
│   │       ├── storage/
│   │       ├── partition/
│   │       ├── error.rs
│   │       └── lib.rs
│   ├── linuxfs-ext/
│   │   └── src/
│   ├── linuxfs-windows/
│   │   └── src/
│   │       ├── disk.rs
│   │       ├── privilege.rs
│   │       └── lib.rs
│   ├── linuxfs-winfsp/
│   │   └── src/
│   ├── linuxfs-config/
│   │   └── src/
│   └── linuxfs-app/
│       └── src/
├── ui/
│   ├── qml/
│   └── resources/
├── tests/
│   ├── images/
│   ├── integration/
│   └── corpus/
├── tools/
└── docs/
```

The exact crate split may be adjusted during implementation, but the boundaries between storage I/O, filesystem parsing, WinFsp, and UI must remain clear.

---

## 9. Filesystem Semantics and Windows Mapping

### 9.1 Permissions

Linux permission bits should be exposed as metadata where useful, but Windows write access must remain denied independent of Linux mode bits.

### 9.2 Ownership

Linux UID/GID values may be exposed in the details UI. V1 does not need to translate Linux identities to Windows accounts.

### 9.3 Symlinks

Symlinks should be supported where WinFsp/Windows semantics can represent them safely.

If a symlink cannot be represented safely, the product must not silently return incorrect file contents.

Path traversal must be normalized and bounded within the mounted filesystem.

### 9.4 Case sensitivity

Linux filesystems are normally case-sensitive. The backend must preserve distinct names and avoid unsafe case-folding assumptions.

Behavior on Windows must be tested for directories containing names differing only by case.

### 9.5 Unsupported Linux file types

Special files such as:

- device nodes
- sockets
- FIFOs

must never cause privileged or device behavior on Windows.

They may be represented as non-openable entries or omitted with a documented policy.

---

## 10. Security and Safety Requirements

### SR-01 — Absolute read-only boundary

V1 production code must not expose source mutation operations.

At the raw-device layer:

- open physical sources with read-only access flags
- open image files read-only
- do not request generic write access
- do not expose a writable block interface

At the filesystem layer:

- use a read-only parser/backend
- provide no write methods in the public V1 filesystem trait

At the WinFsp layer:

- reject every mutating callback

This is defense in depth.

### SR-02 — Untrusted input

Filesystem data is untrusted input.

Code must defend against:

- integer overflows
- malicious offsets
- malformed metadata
- recursive/cyclic metadata
- huge allocation requests
- path traversal
- unexpected encodings
- corrupt directory entries
- denial-of-service images

### SR-03 — Rust safety

Prefer safe Rust.

Any `unsafe` block requires:

- a narrowly defined boundary
- a `// SAFETY:` explanation
- unit/integration coverage around the boundary
- review for lifetime, ownership, aliasing, and Windows API correctness

### SR-04 — No custom kernel driver in V1

Agents/developers shall not create or modify a Windows kernel-mode filesystem driver as part of V1.

WinFsp is the approved driver/framework dependency.

### SR-05 — Corrupt filesystem behavior

LinuxFS Manager is not a recovery tool.

For corrupt metadata:

- do not repair
- do not write
- return a diagnostic error
- mount only if the parser explicitly establishes safe compatibility

---

## 11. Performance Requirements

V1 should target:

- application startup without unnecessary full-disk scans
- asynchronous/background discovery so UI remains responsive
- bounded memory use
- file reads streamed in blocks
- no whole-filesystem loading into RAM for physical disks or large images
- directory enumeration that handles large directories without freezing the UI

Performance is secondary to correctness and safety.

---

## 12. Error Model

Core errors should be structured and categorized.

Suggested categories:

```text
StorageAccess
PermissionDenied
PartitionTable
UnsupportedFilesystem
UnsupportedFeature
FilesystemCorrupt
FilesystemNeedsRecovery
InvalidImage
MountPointUnavailable
WinFspUnavailable
WinFspFailure
Configuration
Internal
```

Every internal error should have:

- technical diagnostic form for logs
- concise user-facing message
- optional remediation hint

Example:

> This Ext filesystem uses a feature that LinuxFS Manager does not currently support. It was not mounted to protect the source filesystem.

---

## 13. Testing Strategy

### 13.1 Unit tests

Cover:

- partition table parsing
- filesystem probing
- offset/range validation
- path normalization
- config serialization/version migration
- error mapping
- Windows-name edge cases

### 13.2 Filesystem image corpus

Maintain generated test images for:

- Ext2
- Ext3
- Ext4
- empty filesystem
- small/large files
- nested directories
- sparse files
- symlinks
- Unicode names
- case-colliding names
- journaled filesystems
- supported feature combinations
- deliberately unsupported incompatible features
- truncated images
- corrupt metadata

Tests should use disposable generated images or redistributable fixtures only.

### 13.3 Read-only regression test

A critical automated test shall:

1. hash the source image
2. mount/read/enumerate it
3. unmount it
4. hash it again
5. require the hashes to match

This test should be part of CI.

### 13.4 Write-denial tests

Through the mounted WinFsp filesystem, test that Windows-side attempts to:

- create
- overwrite
- append
- truncate
- delete
- rename
- create a directory
- change metadata

all fail.

### 13.5 Physical-device testing

Physical-media tests happen only after image-based tests are stable.

Use expendable/test media, never the developer's only copy of important data.

---

## 14. Packaging and Deployment

### 14.1 Executable

Product executable:

`LinuxFSManager.exe`

Repository:

`linuxfs-manager`

### 14.2 Dependencies

The release process must clearly account for:

- Slint runtime components, if not statically bundled
- WinFsp runtime/driver
- Microsoft runtime components if required by the chosen build

The goal is a simple user experience, but licensing and redistribution requirements must be reviewed before bundling dependencies.

### 14.3 Installation modes

Preferred long-term distribution:

- signed Windows installer for normal users
- optional portable ZIP for developers/advanced users when feasible

Portable mode may still require WinFsp to be installed because WinFsp includes Windows driver components.

---

## 15. Configuration File

Preferred V1 format: TOML.

Example:

```toml
config_version = 1
preferred_drive_letter = "L"
log_level = "info"

[ui]
remember_window_position = true

[recent]
image_paths = []
```

No settings are permitted to weaken the V1 read-only guarantee.

There shall be no hidden `enable_write=true` option.

---

## 16. V1 UX Flow

### Physical disk

```text
Launch
  ↓
Discover storage
  ↓
Show Linux candidates
  ↓
Select partition
  ↓
Probe + validate Ext metadata/features
  ↓
Show details
  ↓
Choose mount point
  ↓
Mount READ ONLY
  ↓
Open in Explorer
  ↓
Unmount
```

### Image file

```text
Open Image
  ↓
Open source READ ONLY
  ↓
Detect direct filesystem or partition table
  ↓
Select Ext filesystem
  ↓
Probe + validate
  ↓
Mount READ ONLY
  ↓
Browse/copy
  ↓
Unmount
```

---

## 17. V1 Acceptance Criteria

V1 is considered complete only when:

1. The application builds reproducibly for supported Windows x64 targets.
2. It launches as `LinuxFSManager.exe`.
3. It detects known Ext2/Ext3/Ext4 test volumes.
4. It can open direct filesystem images.
5. It can open supported whole-disk raw images and enumerate Ext partitions.
6. It can read files/directories from the supported image corpus.
7. It can mount supported filesystems through WinFsp.
8. Mounted files can be browsed in Explorer.
9. Files can be copied from the mount to Windows storage.
10. All tested Windows write operations against the mount fail.
11. Source images remain byte-for-byte unchanged after tests.
12. Unsupported incompatible features fail closed.
13. Corrupt test images do not crash or hang the application.
14. Physical supported partitions can be mounted read-only on supported Windows systems.
15. Mounts can be cleanly removed.
16. The app does not require SQLite or another database.
17. Configuration persists in a small versioned file.
18. UI remains responsive during discovery and common reads.
19. No custom kernel driver has been introduced.
20. Automated tests cover the safety-critical read-only boundary.

---

## 18. Roadmap Beyond V1

### V1.x

Potential incremental additions:

- richer Ext/SquashFS/XFS feature coverage
- improved diagnostics
- direct file export from application UI
- checksums for copied files
- VHD/VHDX source support if safe and maintainable
- command-line companion: `linuxfs`

### V2 candidates

- Btrfs read-only backend
- F2FS read-only backend
- LVM discovery/activation in a carefully isolated read-only path
- LUKS support only after an explicit security design
- optional metadata indexing/search if justified

### Write support

Write support is **not automatically a V2 feature**.

It requires a separate PRD, threat/safety analysis, journaling/recovery design, extensive fault-injection tests, and explicit approval before implementation.

---

## 19. Technical Decisions

| Area | V1 decision |
|---|---|
| Product name | LinuxFS Manager |
| OS | Windows 10/11 x64 |
| Core language | Rust |
| GUI | Slint declarative UI |
| UI bridge | Rust models and callbacks |
| FS bridge | WinFsp user-mode filesystem |
| Filesystems | Ext2/Ext3/Ext4, SquashFS, bounded XFS images |
| Access mode | Read-only, mandatory |
| Physical partitions | Yes |
| Raw image files | Yes |
| GPT | Yes |
| MBR | Yes |
| Database | None |
| Persistence | Versioned TOML config |
| Custom kernel driver | No |
| Future FS architecture | Pluggable read-only backends |

---

## 20. Dependency Notes

Dependencies must be pinned deliberately and reviewed before release.

Current architectural candidates:

- **WinFsp** — user-mode filesystem framework for Windows.
- **winfsp-rs** — Rust bindings to WinFsp; validate API maturity and exact supported WinFsp version before committing to it.
- **ext4-view** — Rust read-only Ext filesystem parser; currently documents read-only Ext4 and Ext2 support. Ext3 must be validated using LinuxFS Manager's own test corpus.
- **Slint** — Rust-native declarative UI toolkit used for the Windows desktop shell.

Do not rely on a dependency name alone as proof of filesystem feature compatibility.

---

## 21. References

- WinFsp documentation: https://winfsp.dev/doc/
- WinFsp source/language support: https://winfsp.dev/src/
- Slint documentation: https://docs.slint.dev/
- ext4-view documentation: https://docs.rs/ext4-view/
- Linux kernel Ext4 on-disk format documentation: https://docs.kernel.org/filesystems/ext4/

---

## 22. Open Engineering Questions

These do not block the V1 product definition, but should be resolved during implementation planning:

1. Exact WinFsp Rust binding/API layer to standardize on.
2. Whether physical disk reads live in the GUI process or a narrowly scoped elevated helper.
3. Exact mapping of Linux case-sensitive names and symlinks through WinFsp.
4. Ext3 feature combinations supported by the selected parser.
5. Whether V1 supports MBR extended/logical partitions.
6. Packaging approach for Slint + WinFsp dependencies.
7. Windows ARM64 support after x64 V1.

None of these questions may weaken the mandatory read-only policy.

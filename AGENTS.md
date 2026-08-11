# AGENTS.md — LinuxFS Manager

This file defines repository-wide instructions for coding agents and contributors working on **LinuxFS Manager**.

Read `PRD.md` before planning or modifying product behavior.

---

## 1. Mission

Build a safe Windows application that gives users **read-only** access to Linux Ext2, Ext3, and Ext4 filesystems from:

- physical disks/partitions, and
- raw filesystem/disk image files.

The application uses Rust for core logic, Slint for the desktop UI, and WinFsp for Windows filesystem mounting.

V1 is not a disk editor or filesystem repair utility.

---

## 2. Absolute V1 Safety Rule

> **LinuxFS Manager V1 MUST NEVER WRITE TO THE SOURCE LINUX FILESYSTEM.**

This is the highest-priority repository rule.

It overrides convenience, performance, feature requests, refactoring preferences, and assumptions in third-party examples.

Agents must not add:

- filesystem write APIs
- raw-device write APIs
- write-enabled file handles
- create/delete/rename implementations
- journal replay that modifies the source
- filesystem repair
- formatting
- partition modification
- a hidden/experimental write switch
- write code guarded only by a runtime boolean
- write support under a feature flag
- "temporary" source mutation in tests against non-disposable media

If a requested change conflicts with this rule, stop and surface the conflict rather than implementing it.

---

## 3. Source Access Must Be Read-Only at Every Layer

Defense in depth is required.

### 3.1 Block/device layer

- Open physical devices read-only.
- Open partition views read-only.
- Open image files read-only.
- Do not request generic write access.
- Do not expose `write_at`, `write_block`, or equivalent methods from V1 block abstractions.

A preferred abstraction looks conceptually like:

```rust
pub trait BlockReader {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, dst: &mut [u8]) -> Result<()>;
}
```

### 3.2 Filesystem layer

The public V1 filesystem trait must contain only inspection/read operations.

Do not add mutation methods "for future use."

### 3.3 WinFsp layer

Every mutating Windows filesystem callback must return an appropriate read-only/access-denied result.

Never assume that marking a volume read-only in UI metadata is sufficient.

### 3.4 GUI layer

Do not expose source-side operations such as:

- New Folder
- Rename
- Delete
- Save
- Move into Linux filesystem
- Change permissions
- Change timestamps

"Copy" means copy **from LinuxFS Manager's mounted source to another Windows destination**, not into the source.

---

## 4. V1 Scope

### In scope

- Windows 10/11 x64
- Rust
- Slint declarative UI
- Rust/Slint application bridge
- WinFsp integration
- Ext2
- Ext3
- Ext4
- read-only raw physical disk access
- GPT
- MBR
- raw filesystem images
- raw whole-disk images
- filesystem probing
- filesystem metadata
- mount/unmount
- Explorer access
- configuration file
- logging
- automated image-based tests

### Out of scope

Do not implement unless the PRD is explicitly revised:

- source filesystem writes
- partition editing
- filesystem formatting
- filesystem repair
- file recovery/undelete
- custom Windows kernel filesystem driver
- XFS mounting
- Btrfs mounting
- F2FS mounting
- ZFS mounting
- LUKS decryption
- LVM activation
- MD RAID assembly
- filesystem indexing database
- cloud/network features
- telemetry

Detection/identification of an unsupported format is allowed if implemented safely.

---

## 5. No Database in V1

Do not add SQLite, RocksDB, LevelDB, or another database unless a new requirement clearly justifies it.

V1 persistent state belongs in a small versioned configuration file, preferably:

```text
%APPDATA%\LinuxFS Manager\config.toml
```

Suitable config data:

- UI preferences
- bounded list of recent image paths
- preferred drive letter
- logging preference

Filesystem metadata must normally be read from the filesystem, not copied into a persistent database.

If future requirements introduce large persistent indexes or relational queries, propose SQLite then.

---

## 6. Architecture Boundaries

Keep these concerns isolated:

```text
Slint UI
    ↓
Application/API layer
    ├── Storage discovery
    ├── Mount manager
    ├── Config/logging
    ↓
Read-only BlockReader
    ↓
Filesystem registry
    ↓
ReadOnlyFilesystem
    └── Ext backend (V1)
    ↓
WinFsp adapter
```

Avoid direct dependencies that bypass these boundaries.

### 6.1 UI

Slint UI should:

- render state
- emit user intentions
- display progress/errors

Slint UI should not:

- parse partition tables
- parse Ext structures
- call raw Windows disk APIs
- implement filesystem semantics

### 6.2 Storage

Storage code owns:

- Windows disk enumeration
- safe raw reads
- partition views
- image readers

It does not know about Slint implementation details.

### 6.3 Filesystem backend

Filesystem code consumes a bounded random-access reader.

It should not care whether bytes come from:

- physical media
- a partition
- a raw image

### 6.4 WinFsp adapter

The adapter translates Windows filesystem requests into `ReadOnlyFilesystem` operations.

Do not leak WinFsp types throughout the parser/core crates.

---

## 7. Pluggable Filesystem Design

Although V1 implements Ext only, design against a generic read-only interface.

Conceptually:

```rust
pub trait ReadOnlyFilesystem {
    fn info(&self) -> Result<FilesystemInfo>;
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata>;
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>>;
    fn open_file(&self, path: &FsPath) -> Result<Box<dyn ReadOnlyFile>>;
    fn read_link(&self, path: &FsPath) -> Result<FsPath>;
}
```

Do not add XFS/Btrfs/F2FS implementation code merely to prove the abstraction.

A good abstraction should allow those backends later without speculative implementation today.

---

## 8. Ext Backend Rules

The preferred initial approach is to evaluate/wrap a maintained Rust read-only parser such as `ext4-view`.

Rules:

1. Do not fork/rewrite an Ext implementation before demonstrating a concrete need.
2. Validate Ext3 explicitly with generated test images.
3. Reject unsupported incompatible feature flags.
4. Do not silently ignore parser incompatibility errors.
5. Do not implement journal writeback/replay.
6. Treat all on-disk metadata as untrusted input.
7. Do not load entire large physical filesystems into memory.

Any parser limitation must become a structured compatibility result, not a crash.

---

## 9. WinFsp Rules

Use WinFsp; do not write a custom Windows filesystem kernel driver for V1.

Before implementing a callback:

- determine whether the callback can mutate source state
- if yes, reject it
- if no, translate it through the read-only filesystem API

Keep a test matrix for Windows operations.

At minimum test:

- enumerate directory
- query metadata
- read regular file
- seek/read large file
- symlink behavior
- open missing file
- create attempt → denied
- write attempt → denied
- truncate attempt → denied
- rename attempt → denied
- delete attempt → denied
- mkdir attempt → denied
- metadata mutation attempt → denied

---

## 10. Rust Rules

### 10.1 General

- Prefer safe Rust.
- Use explicit error types.
- Avoid panics for malformed disk/filesystem data.
- Avoid `.unwrap()` / `.expect()` in production paths unless the invariant is local and proven.
- Keep modules focused.
- Use RAII for handles and mounts.
- Prefer immutable data.
- Validate all offset arithmetic.

### 10.2 Integer and offset safety

Disk data is hostile input.

Before reading:

- check addition/multiplication overflow
- verify ranges against source length
- cap allocation sizes
- convert integer widths explicitly and safely

Do not trust filesystem values to fit into `usize`.

### 10.3 `unsafe`

`unsafe` is permitted only when necessary for FFI/Windows APIs.

Every unsafe block must have a nearby comment:

```rust
// SAFETY: ...
unsafe {
    ...
}
```

The explanation must state the invariant that makes the operation safe.

Keep unsafe code behind the smallest possible API boundary.

### 10.4 Concurrency

Do not block the Slint UI thread with:

- disk scanning
- filesystem probing
- large directory enumeration
- physical reads
- mounting operations that may wait

Use worker execution and send bounded state/progress to the UI.

---

## 11. Slint UI Rules

Preferred UI technology:

- Slint declarative UI
- Rust callbacks and application models

Keep the Rust/Slint bridge small and intentional.

Expose application-facing models and commands rather than raw filesystem objects.

Examples of appropriate UI-facing concepts:

```text
StorageSourceViewModel
PartitionViewModel
FilesystemDetailsViewModel
MountStatus
AppCommand
UserFacingError
```

Do not make the UI depend on parser implementation details.

---

## 12. Configuration Rules

V1 configuration is a file, not a database.

Preferred file:

```text
%APPDATA%\LinuxFS Manager\config.toml
```

Requirements:

- include `config_version`
- use serde-compatible typed structures
- default safely when the file is absent
- handle malformed config without crashing
- write atomically
- keep recent lists bounded
- never persist secrets unnecessarily
- never add an option that enables writes

---

## 13. Error Handling

Errors should preserve technical cause while also supporting a clear UI message.

Use categories such as:

```rust
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

Do not turn all failures into strings early.

Do not hide incompatibilities by returning empty directories or partial fake success.

---

## 14. Logging

Use structured logging (`tracing` is a reasonable default unless the project standard changes).

Log:

- source discovery
- probe result
- compatibility decisions
- mount lifecycle
- WinFsp errors
- configuration errors

Avoid logging:

- file contents
- full user directory listings by default
- sensitive file data

Debug logging may include paths only when necessary and documented.

---

## 15. Test-First Safety Workflow

For filesystem work, prefer disposable images before physical devices.

### Mandatory source-integrity regression

For representative test images:

1. hash image
2. execute probe/mount/read workflow
3. unmount
4. hash image
5. assert exact hash equality

A change that causes source-image modification is a release blocker.

### Corruption tests

Malformed images must produce errors, not:

- panic
- process abort
- infinite loop
- huge unbounded allocation
- out-of-range device read

### Fuzzing

Prioritize fuzz/property tests around:

- partition parsing
- superblock parsing
- directory entries
- offset calculations
- path handling

---

## 16. Physical Device Testing

Physical device tests are inherently higher risk.

Rules:

- image tests must pass first
- use expendable/test media
- never use the only copy of important data
- open source device read-only
- verify Windows access flags
- record device identity clearly before tests
- do not automate destructive Windows disk commands

Agents must never suggest `diskpart clean`, format commands, or destructive disk preparation as part of ordinary LinuxFS Manager tests.

---

## 17. Repository Hygiene

Expected root documents:

```text
PRD.md
AGENTS.md
README.md
LICENSE
Cargo.toml
Cargo.lock
```

Keep generated build output out of Git.

Do not commit:

- Slint build directories
- Rust `target/`
- user disk images unless they are intentionally tiny redistributable fixtures
- private filesystem images
- credentials
- local machine paths/config

---

## 18. Dependency Policy

Before adding a dependency:

1. confirm active maintenance or clear rationale
2. inspect license compatibility
3. confirm Windows support where relevant
4. avoid dependencies with unnecessary native/kernel behavior
5. minimize duplicate functionality
6. pin/update deliberately through Cargo

Critical dependencies deserve extra review:

- WinFsp bindings
- filesystem parser
- Windows API wrappers
- Slint bridge

Do not download random filesystem-driver DLLs from third-party sites.

---

## 19. Licensing

Before shipping, review licenses and redistribution terms for:

- WinFsp
- Slint
- selected Rust crates
- installer/bootstrap components

Do not make licensing assumptions based only on "open source" or "free."

Any change in distribution model that affects Slint or WinFsp licensing should be documented.

---

## 20. Packaging Rules

Target product executable:

```text
LinuxFSManager.exe
```

Target repository name:

```text
linuxfs-manager
```

Do not claim the app is completely standalone if WinFsp installation/driver presence is required.

The app should detect missing prerequisites and present a useful remediation path.

---

## 21. Change Discipline

Before coding a feature:

1. check whether it is inside `PRD.md` V1 scope
2. identify affected architecture boundary
3. identify read-only safety impact
4. write/update tests
5. implement smallest viable change
6. run focused tests
7. run workspace tests/checks
8. verify source-integrity tests when storage/filesystem/mount code changed

Do not perform broad unrelated refactors while implementing a focused feature.

---

## 22. Required Checks Before Claiming Completion

Run the checks appropriate to the repository once they exist. The intended baseline is:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
```

When integration tests exist, run them too.

When Slint build tooling is configured, include its build/test checks.

When WinFsp integration changes, run the write-denial test suite.

Never claim "fixed", "complete", or "tests pass" without current command output.

---

## 23. Suggested Implementation Order

Unless the repository already establishes another plan:

1. workspace/bootstrap
2. common errors/types
3. read-only `BlockReader`
4. raw image reader
5. GPT/MBR discovery
6. filesystem probe registry
7. Ext read-only backend
8. image-based integration tests
9. Windows physical-device reader
10. WinFsp adapter
11. mount manager
12. Slint application bridge
13. Slint UI
14. config/logging
15. packaging
16. physical-media validation

Do not start with physical-disk mounting before the image-based core is tested.

---

## 24. Future Work Is Not V1 Work

The architecture anticipates:

- XFS
- Btrfs
- F2FS
- LVM
- LUKS
- MD RAID
- optional indexing
- possibly write support someday

Do not implement these opportunistically.

Especially:

> **Write support requires a separate approved design/PRD.**

Do not create dormant source-writing code in anticipation of it.

---

## 25. Decision Summary

For ambiguity, prefer these defaults:

| Question | Default |
|---|---|
| Can this operation modify the Linux source? | Reject it |
| Unknown Ext incompatible feature? | Reject mount |
| Corrupt metadata? | Return error; do not repair |
| Database needed for settings? | No; use config file |
| Custom kernel driver? | No |
| Full filesystem in memory? | No |
| UI thread for disk operations? | No |
| Image or physical media first for tests? | Image |
| Ext-specific API exposed to UI? | No |
| Add future filesystem backend now? | No |
| Unsure whether an API is read-only? | Verify before use |

---

## 26. Authoritative Project Intent

If implementation code, comments, or third-party examples conflict with `PRD.md` or this file, do not silently follow them.

Surface the conflict and preserve the V1 safety model.

The defining V1 promise is:

> **LinuxFS Manager lets Windows users read Linux Ext2/Ext3/Ext4 filesystems without modifying them.**

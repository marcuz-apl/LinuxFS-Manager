# Read-Only Image Core Design

**Date:** 2026-08-11  
**Status:** Approved for planning  
**Scope:** LinuxFS Manager V1 core milestone

## Goal

Build the first safe, image-first foundation for LinuxFS Manager: a Rust
workspace with a read-only random-access block interface, a raw image reader,
and bounded GPT/MBR partition discovery. This milestone must prove that image
inspection can be performed without modifying the source.

## Scope

Included:

- Rust workspace bootstrap.
- Structured core errors with preserved technical causes.
- Read-only `BlockReader` abstraction.
- Read-only raw image-file reader.
- MBR parsing, including primary and extended/logical partitions.
- GPT parsing, including header and partition-entry CRC validation.
- Checked offset and length arithmetic.
- Synthetic partition fixtures and temporary-file tests.
- Source-image hash equality before and after inspection.

Excluded:

- Physical-device access.
- Ext2/Ext3/Ext4 probing or parsing.
- WinFsp, mounting, or Windows filesystem callbacks.
- Qt/QML and application UI.
- Configuration, logging, packaging, or privilege elevation.
- Any source-writing API, write-enabled handle, or mutation path.

## Architecture

The workspace will contain two focused crates:

```text
linuxfs-manager/
├── Cargo.toml
└── crates/
    ├── linuxfs-core/
    │   └── src/
    │       ├── block.rs
    │       ├── error.rs
    │       ├── partition.rs
    │       └── lib.rs
    └── linuxfs-storage/
        └── src/
            ├── image.rs
            └── lib.rs
```

`linuxfs-core` owns interfaces, checked range operations, partition data
models, and partition parsing. It has no knowledge of Windows files or Qt.

`linuxfs-storage` owns raw image-file access and implements the core reader
interface. It opens image files with read access only and does not expose any
write operation.

The dependency direction is:

```text
linuxfs-storage ──uses──> linuxfs-core
```

The initial implementation will avoid a third-party partition parser. A small
internal CRC-32 routine will be used for GPT validation, with direct tests for
known CRC vectors. This keeps the safety boundary explicit and avoids bringing
an unreviewed filesystem/container abstraction into V1.

## Core interfaces

The public block interface is intentionally read-only:

```rust
pub trait BlockReader {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, buffer: &mut [u8]) -> Result<()>;
}
```

The raw image reader will expose an opening operation equivalent to:

```rust
pub struct RawImageReader { /* private file and length */ }

impl RawImageReader {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self>;
}
```

The reader will use `OpenOptions` with read access enabled and write access
disabled. `read_exact_at` will reject offsets whose requested range exceeds the
source length or overflows `u64`.

Partition discovery will expose validated source layout and partition ranges:

```rust
pub enum SourceLayout {
    DirectImage,
    Mbr { partitions: Vec<Partition> },
    Gpt { partitions: Vec<Partition> },
}

pub struct Partition {
    pub number: u32,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub type_identifier: PartitionType,
}

pub fn discover_layout(reader: &dyn BlockReader) -> Result<SourceLayout>;
```

The exact public field types may use stronger newtypes during implementation,
but callers will receive byte ranges that have already been checked against
the reader length.

## Partition parsing rules

The first milestone uses a 512-byte logical sector for raw image parsing. A
future physical-device reader may provide device-specific sector information,
but it is not part of this milestone.

### MBR

- Read and validate the sector-zero `0x55AA` signature.
- Parse all four primary entries.
- Validate each non-empty entry's start LBA and sector count using checked
  multiplication and addition.
- Reject ranges outside the image instead of truncating them.
- Follow extended-partition EBR chains for logical partitions.
- Bound EBR traversal and reject cycles, repeated offsets, or excessive chain
  length.
- Preserve partition type bytes for later filesystem/container probing.
- Treat malformed partition metadata as a structured `PartitionTable` or
  `InvalidImage` error, never as an empty successful result.

### GPT

- Detect GPT through a protective MBR or the `EFI PART` header signature at
  LBA 1.
- Validate header size, current/backup LBA values, usable-LBA bounds, entry
  count, entry size, and the partition-entry-array range.
- Validate the header CRC with the header CRC field cleared, then validate the
  partition-entry-array CRC.
- Check every non-empty partition range for arithmetic overflow and image
  bounds.
- Preserve partition type and unique GUIDs for later consumers.
- Cap metadata reads and allocations so hostile entry counts or sizes cannot
  create unbounded work.
- Reject malformed or unsupported GPT layouts with a structured error.

If a source has neither a valid MBR nor a valid GPT signature, it is reported
as `DirectImage` for a later filesystem probe. A source with a partition-table
signature but invalid table metadata fails closed rather than falling back to
direct filesystem interpretation.

## Error model

The core error type will preserve a category and technical cause. This
milestone requires at least:

- `StorageAccess`
- `PermissionDenied`
- `InvalidImage`
- `PartitionTable`
- `Internal`

Errors will not be flattened into user-facing strings inside the parser. The
application layer can later map categories to concise messages and remediation
hints.

## Data flow

```text
raw image path
    ↓
RawImageReader::open (read-only handle)
    ↓
BlockReader::len / read_exact_at
    ↓
discover_layout
    ├── DirectImage
    ├── validated MBR partitions
    └── validated GPT partitions
```

No parser will load the entire image. Reads will be bounded to sector-sized
headers and validated partition-entry data.

## Testing strategy

Tests will be written before the implementation for each parser behavior.

Unit tests will cover:

- range addition and multiplication overflow;
- reads at zero, at the final valid byte, and beyond EOF;
- valid and malformed MBR headers;
- primary MBR partitions;
- extended/logical MBR chains;
- EBR cycle and chain-length rejection;
- valid GPT headers and entries;
- GPT header CRC and entry-array CRC failures;
- GPT out-of-range and oversized metadata rejection;
- known CRC-32 vectors;
- direct-image classification.

Integration tests will:

1. Generate a disposable image file containing a valid MBR or GPT fixture.
2. Hash the file before opening it.
3. Open and inspect it through `RawImageReader`.
4. Assert the returned layout and byte ranges.
5. Hash the file afterward.
6. Require exact hash equality.

Malformed fixtures must return errors without panicking, aborting, looping, or
performing out-of-range reads.

## Acceptance criteria

This milestone is complete when:

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   passes.
3. `cargo test --workspace` passes.
4. `cargo build --workspace` passes.
5. Valid MBR and GPT image fixtures return correct, in-bounds byte ranges.
6. Extended/logical MBR traversal is bounded and cycle-safe.
7. GPT header and entry-array CRC failures are rejected.
8. Malformed images produce structured errors rather than panics or hangs.
9. Image hashes are unchanged after all inspection tests.
10. No public API exposes source mutation or write access.

## Next milestone

After this core passes, the next milestone will add filesystem probing and the
generic read-only filesystem interface, followed by the Ext backend. Physical
devices, WinFsp, and the Qt application remain later milestones and must not be
pulled into this foundation work.

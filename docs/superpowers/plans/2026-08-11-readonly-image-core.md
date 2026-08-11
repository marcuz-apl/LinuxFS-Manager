# Read-Only Image Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first LinuxFS Manager milestone: a Rust workspace that opens raw images read-only and safely discovers direct images, MBR partitions, extended/logical MBR partitions, and GPT partitions without modifying source bytes.

**Architecture:** `linuxfs-core` owns typed errors, the thread-safe read-only block contract, checked geometry/ranges, partition models, and bounded MBR/GPT parsers. `linuxfs-storage` owns the Windows raw-image file handle and implements positional reads; it depends on `linuxfs-core`, while the core remains independent of Windows, Qt, WinFsp, and filesystem parsers.

**Tech Stack:** Rust 1.97.1, Rust 2024 edition, Cargo workspace resolver 3, Rust standard library only, Windows 10/11 x64.

## Global Constraints

- LinuxFS Manager V1 MUST NEVER WRITE TO THE SOURCE LINUX FILESYSTEM.
- No public source-writing API, write-enabled source handle, journal replay, repair, formatting, or partition mutation is permitted.
- `BlockReader` is read-only and must be `Send + Sync`.
- Every offset addition and LBA multiplication is checked before use.
- Raw image sources use a documented 512-byte logical-sector default; supplied geometry is validated and unsupported geometry is rejected.
- Unknown or malformed partition metadata fails closed with a structured error.
- Parsers use bounded reads and bounded allocations; they never load the entire image.
- Production paths contain no `.unwrap()`, `.expect()`, or undocumented `unsafe` blocks.
- Tests create only disposable synthetic images and must prove source bytes are unchanged after inspection.
- Milestone 1 adds no Ext parser, physical-device access, WinFsp, Qt/QML, configuration, logging, packaging, privilege elevation, database, or supported CLI.
- The `.githooks/pre-commit` hook automatically bumps and stages `VERSION`; implementation commits must not edit `VERSION` manually.

## File Map

- Create `Cargo.toml`: workspace membership, Rust version, edition, and workspace lints.
- Create `.gitignore`: ignore only generated Rust build output for this milestone.
- Create `crates/linuxfs-core/Cargo.toml`: dependency-free core crate manifest.
- Create `crates/linuxfs-core/src/lib.rs`: public core module exports.
- Create `crates/linuxfs-core/src/error.rs`: structured error category, technical message/source, and `Result<T>` alias.
- Create `crates/linuxfs-core/src/block.rs`: `BlockReader`, `BlockGeometry`, and checked read-range validation.
- Create `crates/linuxfs-core/src/partition.rs`: public partition models and top-level layout discovery.
- Create `crates/linuxfs-core/src/partition/mbr.rs`: primary MBR and bounded EBR-chain parsing.
- Create `crates/linuxfs-core/src/partition/gpt.rs`: GPT header, entry-array, and partition validation.
- Create `crates/linuxfs-core/src/partition/crc32.rs`: internal IEEE CRC-32 implementation.
- Create `crates/linuxfs-storage/Cargo.toml`: storage crate manifest depending only on `linuxfs-core`.
- Create `crates/linuxfs-storage/src/lib.rs`: storage exports.
- Create `crates/linuxfs-storage/src/image.rs`: read-only Windows raw-image reader.
- Create `crates/linuxfs-storage/tests/source_integrity.rs`: end-to-end synthetic-image integrity regression.
- Modify `README.md`: report the verified Milestone 1 capability and explicit exclusions.
- Generate `Cargo.lock` with Cargo and commit it.

---

### Task 1: Workspace and read-only core contracts

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `crates/linuxfs-core/Cargo.toml`
- Create: `crates/linuxfs-core/src/lib.rs`
- Create: `crates/linuxfs-core/src/error.rs`
- Create: `crates/linuxfs-core/src/block.rs`
- Generate: `Cargo.lock`

**Interfaces:**
- Consumes: Rust 1.97.1 standard library.
- Produces: `ErrorCategory`, `Error`, `Result<T>`, `BlockReader`, `BlockGeometry`, and `validate_read_range`.

- [ ] **Step 1: Create the workspace manifest and failing contract tests**

Create `Cargo.toml`:

```toml
[workspace]
members = ["crates/*"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.97"

[workspace.lints.rust]
unsafe_code = "deny"
```

Create `.gitignore` containing `/target/`. Create the core manifest with `publish = false` and `lints.workspace = true`:

```toml
[package]
name = "linuxfs-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
publish = false

[lints]
workspace = true
```

Export `block` and `error` from `lib.rs`. In otherwise empty `error.rs` and `block.rs`, add tests that express the desired API:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_preserves_category_and_message() {
        let error = Error::new(ErrorCategory::InvalidImage, "bad image");
        assert_eq!(error.category(), ErrorCategory::InvalidImage);
        assert_eq!(error.to_string(), "bad image");
    }
}
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_accepts_supported_sector_sizes() {
        let sector_512 = BlockGeometry::new(512).expect("512-byte sectors are supported");
        let sector_4096 = BlockGeometry::new(4096).expect("4096-byte sectors are supported");
        assert_eq!(sector_512.logical_sector_size(), 512);
        assert_eq!(sector_4096.logical_sector_size(), 4096);
    }

    #[test]
    fn geometry_rejects_zero_non_power_of_two_and_excessive_values() {
        for value in [0, 513, 131_072] {
            assert_eq!(BlockGeometry::new(value).map_err(|error| error.category()), Err(ErrorCategory::InvalidImage));
        }
    }

    #[test]
    fn read_range_rejects_overflow_and_end_past_source() {
        assert!(validate_read_range(16, u64::MAX, 1).is_err());
        assert!(validate_read_range(16, 15, 2).is_err());
        assert!(validate_read_range(16, 16, 0).is_ok());
    }
}
```

- [ ] **Step 2: Run the focused tests and verify the intended failure**

Run: `cargo test -p linuxfs-core`

Expected: compilation fails because `Error`, `ErrorCategory`, `BlockGeometry`, and `validate_read_range` do not exist. Fix only test syntax if the failure is unrelated to those missing contracts.

- [ ] **Step 3: Implement the minimal structured error type**

Implement these exact public shapes in `error.rs`:

```rust
use std::{error::Error as StdError, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    StorageAccess,
    PermissionDenied,
    InvalidImage,
    PartitionTable,
    Internal,
}

#[derive(Debug)]
pub struct Error {
    category: ErrorCategory,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self { category, message: message.into(), source: None }
    }

    pub fn with_source(
        category: ErrorCategory,
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self { category, message: message.into(), source: Some(Box::new(source)) }
    }

    pub const fn category(&self) -> ErrorCategory { self.category }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_deref().map(|source| source as &(dyn StdError + 'static))
    }
}
```

- [ ] **Step 4: Implement the minimal block contract and checked geometry/range helpers**

Implement in `block.rs`:

```rust
use crate::error::{Error, ErrorCategory, Result};

pub const RAW_IMAGE_LOGICAL_SECTOR_SIZE: u32 = 512;
const MAX_LOGICAL_SECTOR_SIZE: u32 = 65_536;

pub trait BlockReader: Send + Sync {
    fn len(&self) -> Result<u64>;
    fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockGeometry { logical_sector_size: u32 }

impl BlockGeometry {
    pub fn new(logical_sector_size: u32) -> Result<Self> {
        if logical_sector_size < RAW_IMAGE_LOGICAL_SECTOR_SIZE
            || logical_sector_size > MAX_LOGICAL_SECTOR_SIZE
            || !logical_sector_size.is_power_of_two()
        {
            return Err(Error::new(ErrorCategory::InvalidImage, "unsupported logical sector size"));
        }
        Ok(Self { logical_sector_size })
    }

    pub const fn raw_image_512() -> Self { Self { logical_sector_size: 512 } }
    pub const fn logical_sector_size(self) -> u32 { self.logical_sector_size }
}

pub fn validate_read_range(source_len: u64, offset: u64, requested_len: usize) -> Result<()> {
    let requested_len = u64::try_from(requested_len)
        .map_err(|_| Error::new(ErrorCategory::StorageAccess, "read length does not fit u64"))?;
    let end = offset.checked_add(requested_len)
        .ok_or_else(|| Error::new(ErrorCategory::StorageAccess, "read range overflow"))?;
    if end > source_len {
        return Err(Error::new(ErrorCategory::StorageAccess, "read extends beyond source"));
    }
    Ok(())
}
```

Re-export the public types from `lib.rs` without exposing any write trait or write method.

- [ ] **Step 5: Run formatting and the focused tests**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core`

Expected: all core contract tests pass and Cargo creates `Cargo.lock`.

- [ ] **Step 6: Commit the workspace and contracts**

```powershell
git add Cargo.toml Cargo.lock .gitignore crates/linuxfs-core
git commit -m "feat: add read-only core contracts"
```

Expected hook behavior: `VERSION` is bumped and staged automatically.

---

### Task 2: Read-only Windows raw-image reader

**Files:**
- Create: `crates/linuxfs-storage/Cargo.toml`
- Create: `crates/linuxfs-storage/src/lib.rs`
- Create: `crates/linuxfs-storage/src/image.rs`

**Interfaces:**
- Consumes: `linuxfs_core::{BlockReader, Error, ErrorCategory, Result, validate_read_range}`.
- Produces: `RawImageReader::open(path)`, `BlockReader::len`, and positional `BlockReader::read_exact_at`.

- [ ] **Step 1: Add the storage manifest and failing image-reader tests**

Use this manifest:

```toml
[package]
name = "linuxfs-storage"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
publish = false

[dependencies]
linuxfs-core = { path = "../linuxfs-core" }

[lints]
workspace = true
```

Export `RawImageReader` from `lib.rs`. Add unit tests in `image.rs` using a test-only temporary file built from `std::env::temp_dir`, the process ID, and an `AtomicU64`. The helper removes its exact file in `Drop`. Test these behaviors:

```rust
#[test]
fn reads_at_zero_and_the_final_valid_byte() {
    let image = TempImage::new(&[10, 20, 30, 40]);
    let reader = RawImageReader::open(image.path()).expect("test image opens");
    let mut first = [0; 2];
    reader.read_exact_at(0, &mut first).expect("first bytes read");
    assert_eq!(first, [10, 20]);
    let mut last = [0; 1];
    reader.read_exact_at(3, &mut last).expect("last byte reads");
    assert_eq!(last, [40]);
}

#[test]
fn rejects_reads_past_end_without_changing_destination() {
    let image = TempImage::new(&[1, 2, 3, 4]);
    let reader = RawImageReader::open(image.path()).expect("test image opens");
    let mut destination = [0xA5; 2];
    let error = reader.read_exact_at(3, &mut destination).expect_err("range is rejected");
    assert_eq!(error.category(), ErrorCategory::StorageAccess);
    assert_eq!(destination, [0xA5; 2]);
}
```

Also test an empty destination at EOF, a missing path (`StorageAccess`), and a read-only Windows file attribute to prove opening does not request write access.

- [ ] **Step 2: Run the storage tests and verify the intended failure**

Run: `cargo test -p linuxfs-storage image::tests`

Expected: compilation fails because `RawImageReader` is not defined.

- [ ] **Step 3: Implement read-only open and positional exact reads**

Use `OpenOptions::new().read(true).write(false).create(false)` and cache `metadata().len()`. On Windows, use `std::os::windows::fs::FileExt::seek_read` in a loop:

```rust
pub struct RawImageReader {
    file: std::fs::File,
    len: u64,
}

impl RawImageReader {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(path.as_ref())
            .map_err(map_open_error)?;
        let len = file.metadata()
            .map_err(|source| Error::with_source(ErrorCategory::StorageAccess, "cannot read image metadata", source))?
            .len();
        Ok(Self { file, len })
    }
}
```

`read_exact_at` must call `validate_read_range` before the first OS read, return immediately for an empty destination, retry `Interrupted`, treat a zero-byte early read as `StorageAccess`, and map `PermissionDenied` separately. It must not expose the private `File` or any write method.

- [ ] **Step 4: Run focused tests and Clippy**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-storage image::tests`

Run: `cargo clippy -p linuxfs-storage --all-targets -- -D warnings`

Expected: all image-reader tests pass with no warnings.

- [ ] **Step 5: Commit the raw-image reader**

```powershell
git add Cargo.lock crates/linuxfs-storage
git commit -m "feat: add read-only raw image reader"
```

---
### Task 3: Partition models and direct-image classification

**Files:**
- Create: `crates/linuxfs-core/src/partition.rs`
- Modify: `crates/linuxfs-core/src/lib.rs`

**Interfaces:**
- Consumes: `BlockReader`, `BlockGeometry`, and structured core errors.
- Produces: `Partition`, `PartitionType`, `SourceLayout`, and `discover_layout(reader, geometry)`.

- [ ] **Step 1: Write failing tests for the public partition model and direct images**

Add a test-only `MemoryReader { bytes: Vec<u8> }` implementing `BlockReader`. Add tests in `partition.rs`:

```rust
#[test]
fn source_without_partition_signatures_is_direct_image() {
    let reader = MemoryReader::new(vec![0; 4096]);
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("unsigned source is a direct image");
    assert!(matches!(layout, SourceLayout::DirectImage));
}

#[test]
fn short_source_is_direct_image_without_out_of_range_read() {
    let reader = MemoryReader::new(vec![0; 100]);
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("short source is a direct image");
    assert!(matches!(layout, SourceLayout::DirectImage));
}

#[test]
fn partition_signature_fails_closed_until_validated() {
    let mut bytes = vec![0; 4096];
    bytes[510..512].copy_from_slice(&[0x55, 0xAA]);
    let error = discover_layout(&MemoryReader::new(bytes), BlockGeometry::raw_image_512())
        .expect_err("unvalidated table is rejected");
    assert_eq!(error.category(), ErrorCategory::PartitionTable);
}
```

- [ ] **Step 2: Run the focused test and verify the intended failure**

Run: `cargo test -p linuxfs-core partition::tests`

Expected: compilation fails because the partition models and `discover_layout` do not exist.

- [ ] **Step 3: Implement the public models with no mutation API**

Use these exact shapes:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceLayout {
    DirectImage,
    Mbr { partitions: Vec<Partition> },
    Gpt { partitions: Vec<Partition> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub number: u32,
    pub byte_offset: u64,
    pub byte_length: u64,
    pub type_identifier: PartitionType,
    pub unique_identifier: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionType {
    Mbr(u8),
    Gpt([u8; 16]),
}
```

Do not add setters or write operations. `discover_layout` must:

1. Query source length.
2. Return `DirectImage` when fewer than 512 bytes are available.
3. Read at most one validated logical sector at LBA 0 and one at LBA 1.
4. Return `DirectImage` only when neither `0x55AA` nor `EFI PART` is present.
5. Return `PartitionTable` while a recognized table has not yet been fully validated.

- [ ] **Step 4: Run focused and workspace tests**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core partition::tests`

Run: `cargo test --workspace`

Expected: direct and short images classify successfully; recognized but unvalidated partition signatures fail closed.

- [ ] **Step 5: Commit partition models and direct detection**

```powershell
git add crates/linuxfs-core/src
git commit -m "feat: classify direct disk images"
```

---

### Task 4: Primary MBR partition parsing

**Files:**
- Create: `crates/linuxfs-core/src/partition/mbr.rs`
- Modify: `crates/linuxfs-core/src/partition.rs`

**Interfaces:**
- Consumes: a validated sector-zero buffer, `BlockReader`, `BlockGeometry`, and partition models.
- Produces: `mbr::parse(reader, geometry, sector_zero) -> Result<Vec<Partition>>` for non-extended primary entries.

- [ ] **Step 1: Write failing primary-MBR behavior tests**

Build a synthetic image in memory by placing the `0x55AA` signature at bytes 510–511 and four 16-byte entries at byte 446. Add focused tests for:

```rust
#[test]
fn discovers_primary_linux_partition() {
    let reader = mbr_image(&[(0, 0x83, 2, 4)], 16);
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("valid MBR parses");
    assert_eq!(layout, SourceLayout::Mbr { partitions: vec![Partition {
        number: 1,
        byte_offset: 1024,
        byte_length: 2048,
        type_identifier: PartitionType::Mbr(0x83),
        unique_identifier: None,
    }] });
}

#[test]
fn rejects_primary_partition_past_source_end() {
    let reader = mbr_image(&[(0, 0x83, 15, 2)], 16);
    let error = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect_err("out-of-bounds partition is rejected");
    assert_eq!(error.category(), ErrorCategory::PartitionTable);
}
```

Also test an empty valid MBR, inconsistent empty entries (`type == 0` with nonzero range and nonzero type with zero sectors), multiple ordinary primary entries, and a protective `0xEE` entry without a GPT header.

- [ ] **Step 2: Run the primary-MBR tests and verify the intended failure**

Run: `cargo test -p linuxfs-core partition::mbr::tests`

Expected: tests fail because `mbr::parse` is absent and `discover_layout` still rejects every MBR signature.

- [ ] **Step 3: Implement bounded primary-entry parsing**

In `mbr.rs`, define:

```rust
const PARTITION_TABLE_OFFSET: usize = 446;
const PARTITION_ENTRY_SIZE: usize = 16;
const PRIMARY_ENTRY_COUNT: usize = 4;
const PROTECTIVE_GPT_TYPE: u8 = 0xEE;
const EXTENDED_TYPES: [u8; 3] = [0x05, 0x0F, 0x85];

pub(super) fn parse(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    sector_zero: &[u8],
) -> Result<Vec<Partition>>;
```

For each entry, decode `type` at byte 4, start LBA at bytes 8–11, and sector count at bytes 12–15 as little-endian values. A helper must use `checked_mul` and `checked_add`, then verify the byte range against `reader.len()`. Number ordinary primary entries using their one-based table slot. Reject protective GPT entries in this parser and reject extended entries until Task 5. Do not truncate, skip, or repair malformed entries.

Update `discover_layout` so a GPT signature at LBA 1 still fails closed, while an MBR signature delegates to `mbr::parse` and returns `SourceLayout::Mbr` only after validation.

- [ ] **Step 4: Run focused tests, all core tests, and Clippy**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core partition::mbr::tests`

Run: `cargo test -p linuxfs-core`

Run: `cargo clippy -p linuxfs-core --all-targets -- -D warnings`

Expected: primary-MBR cases pass, malformed entries fail with `PartitionTable`, and no warnings are emitted.

- [ ] **Step 5: Commit primary MBR support**

```powershell
git add crates/linuxfs-core/src/partition.rs crates/linuxfs-core/src/partition/mbr.rs
git commit -m "feat: parse primary MBR partitions"
```

---

### Task 5: Extended and logical MBR partitions

**Files:**
- Modify: `crates/linuxfs-core/src/partition/mbr.rs`

**Interfaces:**
- Consumes: one validated extended primary entry and read-only sector access.
- Produces: logical partitions numbered from 5, with the extended container omitted from returned data.

- [ ] **Step 1: Write failing EBR-chain tests**

Create an MBR whose extended container begins at LBA 1. Put the first EBR at LBA 1, its logical partition one LBA after that EBR, and its next-link entry relative to the extended base. Add tests for:

```rust
#[test]
fn follows_two_logical_partitions() {
    let reader = two_logical_partition_image();
    let SourceLayout::Mbr { partitions } = discover_layout(
        &reader,
        BlockGeometry::raw_image_512(),
    ).expect("valid EBR chain parses") else {
        panic!("expected MBR layout");
    };
    assert_eq!(partitions.iter().map(|partition| partition.number).collect::<Vec<_>>(), vec![5, 6]);
    assert_eq!(partitions[0].byte_offset, 2 * 512);
    assert_eq!(partitions[1].byte_offset, 6 * 512);
}

#[test]
fn rejects_repeated_ebr_offset() {
    let reader = cyclic_ebr_image();
    let error = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect_err("EBR cycle is rejected");
    assert_eq!(error.category(), ErrorCategory::PartitionTable);
}
```

Also test: more than one extended primary, an EBR without `0x55AA`, a logical partition outside its extended container, a next-link outside the container, unexpected nonempty third/fourth EBR entries, and a chain containing 129 EBRs.

- [ ] **Step 2: Run the EBR tests and verify the intended failure**

Run: `cargo test -p linuxfs-core partition::mbr::tests`

Expected: new tests fail because extended entries are still rejected.

- [ ] **Step 3: Implement a bounded, cycle-safe EBR walker**

Use these limits and state:

```rust
const MAX_EBR_CHAIN_LENGTH: usize = 128;

fn parse_ebr_chain(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    extended_base_lba: u64,
    extended_sector_count: u64,
) -> Result<Vec<Partition>>;
```

Track visited EBR LBAs in `std::collections::HashSet<u64>`. The first EBR entry is relative to the current EBR; the second link is relative to the original extended base. Validate every EBR sector, logical partition, and next-link against both the source length and the extended-container range. Stop only on an empty second entry. Reject a repeat before reading it and reject a 129th iteration. Return ordinary primary partitions first, followed by logical partitions in chain order; never return the extended container itself.

- [ ] **Step 4: Run the complete MBR test group**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core partition::mbr::tests`

Run: `cargo clippy -p linuxfs-core --all-targets -- -D warnings`

Expected: valid logical chains pass; cycles, excessive chains, malformed EBRs, and escaped ranges fail quickly with `PartitionTable`.

- [ ] **Step 5: Commit logical MBR support**

```powershell
git add crates/linuxfs-core/src/partition/mbr.rs
git commit -m "feat: parse logical MBR partitions"
```

---
### Task 6: IEEE CRC-32 and GPT header validation

**Files:**
- Create: `crates/linuxfs-core/src/partition/crc32.rs`
- Create: `crates/linuxfs-core/src/partition/gpt.rs`
- Modify: `crates/linuxfs-core/src/partition.rs`

**Interfaces:**
- Consumes: read-only sector data, source length, and `BlockGeometry`.
- Produces: internal `crc32::ieee(bytes) -> u32` and a validated internal GPT `Header`.

- [ ] **Step 1: Write failing CRC and GPT-header tests**

In `crc32.rs`, add independent known-vector tests:

```rust
#[test]
fn matches_ieee_known_vectors() {
    assert_eq!(ieee(b""), 0x0000_0000);
    assert_eq!(ieee(b"123456789"), 0xCBF4_3926);
}
```

In `gpt.rs`, create a 512-byte header fixture with `EFI PART`, revision `0x0001_0000`, header size 92, current LBA 1, backup LBA 127, usable LBAs 34–126, entry array at LBA 2, one 128-byte entry, and a valid recomputed header CRC. Add tests that accept this header and reject each of these independent corruptions:

- header size below 92 or above the logical sector size;
- wrong revision or nonzero reserved field;
- current LBA other than 1;
- backup or usable LBA outside the source;
- first usable LBA greater than last usable LBA;
- entry size below 128, not divisible by 128, or above 4096;
- zero entry count, more than 16,384 entries, or an entry array above 16 MiB;
- a changed header byte without a recomputed CRC.

The CRC test must expect `PartitionTable` rather than a panic.

- [ ] **Step 2: Run focused GPT tests and verify the intended failure**

Run: `cargo test -p linuxfs-core partition::crc32::tests`

Run: `cargo test -p linuxfs-core partition::gpt::tests`

Expected: compilation fails because `ieee` and GPT header parsing are absent.

- [ ] **Step 3: Implement the internal IEEE CRC-32 routine**

Use a small table-free implementation with polynomial `0xEDB8_8320`, initial value `0xFFFF_FFFF`, and final bitwise inversion:

```rust
pub(super) fn ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
```

- [ ] **Step 4: Implement bounded GPT header parsing**

Use these internal constants and shape:

```rust
const GPT_SIGNATURE: &[u8; 8] = b"EFI PART";
const GPT_REVISION_1_0: u32 = 0x0001_0000;
const MIN_HEADER_SIZE: usize = 92;
const MIN_ENTRY_SIZE: u32 = 128;
const MAX_ENTRY_SIZE: u32 = 4096;
const MAX_ENTRY_COUNT: u32 = 16_384;
const MAX_ENTRY_ARRAY_BYTES: u64 = 16 * 1024 * 1024;

struct Header {
    first_usable_lba: u64,
    last_usable_lba: u64,
    entries_lba: u64,
    entry_count: u32,
    entry_size: u32,
    entries_crc32: u32,
}
```

`parse_header(source_len, geometry, sector)` must require the source length to be an exact multiple of logical sector size, validate every listed field with checked arithmetic, copy only `header_size` bytes for CRC calculation, zero bytes 16–19 in that copy, and compare the computed CRC before returning `Header`. The maximum copied header is one validated sector (65,536 bytes).

Keep GPT discovery fail-closed at the top level until entry parsing is complete in Task 7.

- [ ] **Step 5: Run focused tests and core Clippy**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core partition::crc32::tests`

Run: `cargo test -p linuxfs-core partition::gpt::tests`

Run: `cargo clippy -p linuxfs-core --all-targets -- -D warnings`

Expected: known CRC vectors and all GPT header acceptance/rejection cases pass without warnings.

- [ ] **Step 6: Commit CRC and GPT header validation**

```powershell
git add crates/linuxfs-core/src/partition.rs crates/linuxfs-core/src/partition
git commit -m "feat: validate GPT headers"
```

---

### Task 7: GPT partition-entry parsing and layout discovery

**Files:**
- Modify: `crates/linuxfs-core/src/partition/gpt.rs`
- Modify: `crates/linuxfs-core/src/partition.rs`

**Interfaces:**
- Consumes: validated GPT `Header`, `BlockReader`, and `BlockGeometry`.
- Produces: `gpt::parse(reader, geometry, header_sector) -> Result<Vec<Partition>>` and `SourceLayout::Gpt`.

- [ ] **Step 1: Write failing GPT entry-array tests**

Extend the fixture builder to place a one-entry array at LBA 2. The nonempty entry contains a type GUID in bytes 0–15, unique GUID in bytes 16–31, first LBA 40, last LBA 41, and zero attributes/name bytes. Recompute the entry-array CRC and then the header CRC. Assert:

```rust
#[test]
fn discovers_valid_gpt_partition() {
    let reader = one_partition_gpt_image();
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("valid GPT parses");
    let SourceLayout::Gpt { partitions } = layout else {
        panic!("expected GPT layout");
    };
    assert_eq!(partitions.len(), 1);
    assert_eq!(partitions[0].number, 1);
    assert_eq!(partitions[0].byte_offset, 40 * 512);
    assert_eq!(partitions[0].byte_length, 2 * 512);
    assert!(matches!(partitions[0].type_identifier, PartitionType::Gpt(_)));
    assert!(partitions[0].unique_identifier.is_some());
}
```

Add independent rejection tests for a changed entry byte without updated CRC, zero unique GUID on a nonempty entry, first LBA greater than last LBA, an entry outside the usable range, an entry range past the source, an overflowing entry-array range, and an entry array CRC mismatch. Also test empty entries and GPT detection at LBA 1 without an MBR signature.

- [ ] **Step 2: Run focused GPT tests and verify the intended failure**

Run: `cargo test -p linuxfs-core partition::gpt::tests`

Expected: the valid-layout test fails because top-level GPT discovery remains fail-closed and entries are not parsed.

- [ ] **Step 3: Implement bounded entry-array reading and validation**

Implement:

```rust
pub(super) fn parse(
    reader: &dyn BlockReader,
    geometry: BlockGeometry,
    header_sector: &[u8],
) -> Result<Vec<Partition>>;
```

After `parse_header` succeeds:

1. Compute `entry_count * entry_size` with checked multiplication and enforce the 16 MiB cap before converting to `usize`.
2. Compute `entries_lba * logical_sector_size` and the ending byte with checked arithmetic.
3. Verify the complete array is inside the source, allocate exactly the validated size, and perform one bounded `read_exact_at`.
4. Validate `crc32::ieee(&entry_array)` against the header field.
5. Iterate exactly `entry_count` fixed-size records; skip only entries with an all-zero type GUID.
6. Require a nonzero unique GUID for every nonempty entry.
7. Validate `first_lba <= last_lba`, both LBAs inside the usable range, and the inclusive byte length using checked `last - first + 1` arithmetic.
8. Return `PartitionType::Gpt(type_guid)`, `Some(unique_guid)`, and an entry-index-based one-based partition number.

No GPT name decoding is required in Milestone 1; the parser still validates and bounds the full record before ignoring name bytes.

- [ ] **Step 4: Connect GPT-first discovery semantics**

Update `discover_layout` to probe `EFI PART` at LBA 1 before choosing MBR. A valid GPT returns `SourceLayout::Gpt`. A protective MBR without a GPT header returns `PartitionTable`. A malformed GPT header or array never falls back to MBR or direct-image classification.

- [ ] **Step 5: Run all partition tests and workspace Clippy**

Run: `cargo fmt --all`

Run: `cargo test -p linuxfs-core partition`

Run: `cargo test --workspace`

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: valid MBR, EBR, GPT, and direct-image fixtures pass; every malformed fixture returns a structured error; Clippy emits no warning.

- [ ] **Step 6: Commit complete GPT discovery**

```powershell
git add crates/linuxfs-core/src/partition.rs crates/linuxfs-core/src/partition/gpt.rs
git commit -m "feat: parse GPT partitions"
```

---

### Task 8: Source-integrity regression, documentation, and milestone verification

**Files:**
- Create: `crates/linuxfs-storage/tests/source_integrity.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: `RawImageReader`, `BlockGeometry::raw_image_512`, and `discover_layout`.
- Produces: an end-to-end proof that opening and inspecting a source image does not alter it.

- [ ] **Step 1: Write the failing end-to-end integrity test**

Create a disposable 16-sector MBR image with one Linux partition. The test helper may write the fixture before inspection, but production code never receives a write handle. Use `std::collections::hash_map::DefaultHasher` to hash bytes in fixed-size chunks before and after inspection, and retain the complete small fixture bytes for an authoritative exact comparison:

```rust
#[test]
fn inspection_preserves_source_image_exactly() {
    let image = TempImage::mbr_with_linux_partition();
    let bytes_before = std::fs::read(image.path()).expect("fixture reads before inspection");
    let hash_before = hash_file(image.path()).expect("fixture hashes before inspection");

    let reader = RawImageReader::open(image.path()).expect("image opens read-only");
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("layout discovery succeeds");
    assert!(matches!(layout, SourceLayout::Mbr { .. }));
    drop(reader);

    let hash_after = hash_file(image.path()).expect("fixture hashes after inspection");
    let bytes_after = std::fs::read(image.path()).expect("fixture reads after inspection");
    assert_eq!(hash_after, hash_before);
    assert_eq!(bytes_after, bytes_before);
}
```

Add a second test that runs malformed-MBR and malformed-GPT discovery through `std::panic::catch_unwind`, asserts an error result, and confirms exact bytes remain unchanged.

- [ ] **Step 2: Run the integration test and verify the intended failure**

Run: `cargo test -p linuxfs-storage --test source_integrity`

Expected: the new test initially fails because its fixture/hash helpers and integration wiring are incomplete; finish only test support until the failure reaches the production behavior under test.

- [ ] **Step 3: Complete the minimal test support and make the integrity test pass**

Implement `TempImage` entirely inside the integration test using a unique path in `std::env::temp_dir`, process ID, and `AtomicU64`; remove only that exact path in `Drop`. `hash_file` opens read-only and streams through an 8 KiB buffer. Do not add `tempfile`, `sha2`, or another dependency; exact byte equality is the authoritative integrity assertion.

If the test exposes a production defect, first preserve the failing test, then make the smallest parser/reader correction and rerun the focused unit test that owns the defect before rerunning this integration test.

- [ ] **Step 4: Update README with verified scope**

Replace the bootstrap-only status with a concise Milestone 1 section stating that the repository now provides read-only raw-image access, direct/MBR/GPT discovery, bounded logical MBR traversal, GPT CRC validation, and source-integrity regression tests. Keep Ext, physical devices, mounting, and UI explicitly marked as subsequent milestones; do not claim the V1 product is complete.

- [ ] **Step 5: Run the complete required verification gate**

Run each command fresh and inspect its exit code and full output:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
git diff --check
```

Then inspect production-source hits:

```powershell
rg -n "write_at|write_block|write\(true\)|create\(true\)|\.unwrap\(|\.expect\(" crates -g "*.rs"
```

Expected: no source-writing API or write-enabled production handle exists. Any `unwrap`/`expect` hits are confined to `#[cfg(test)]` code and are reviewed manually. All required Cargo commands exit 0.

- [ ] **Step 6: Review the milestone acceptance criteria against evidence**

Confirm from current output that valid MBR/GPT ranges are correct, EBR traversal is bounded and cycle-safe, GPT CRC failures are rejected, malformed images return errors without panic/hang, source bytes remain exactly equal, and no public mutation API exists. Record any unmet item as incomplete rather than weakening the criterion.

- [ ] **Step 7: Commit the integrity test and verified documentation**

```powershell
git add crates/linuxfs-storage/tests/source_integrity.rs README.md
git commit -m "test: verify image inspection integrity"
```

- [ ] **Step 8: Re-run the full verification after the commit hook**

The commit hook changes `VERSION`; therefore rerun:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
git status --short
```

Expected: all checks exit 0 and the working tree is clean.

## Plan Coverage

- Workspace/bootstrap: Task 1.
- Structured errors and checked block geometry/ranges: Task 1.
- Read-only raw-image handle and exact positional reads: Task 2.
- Direct-image classification and public partition models: Task 3.
- Primary MBR parsing and malformed-range rejection: Task 4.
- Bounded, cycle-safe EBR traversal: Task 5.
- Known CRC vectors and GPT header validation: Task 6.
- GPT entry-array CRC, allocation caps, GUID/range preservation: Task 7.
- Disposable end-to-end image hash and exact-byte integrity: Task 8.
- Formatting, lint, tests, build, API audit, and clean-tree evidence: Task 8.

This plan intentionally contains no work from Milestone 2 or any native UI/mount milestone.

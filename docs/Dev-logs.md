# LinuxFS Manager development log

## Implementation history summary

- **Read-only core:** established the Rust workspace, typed errors, bounded
  `BlockReader`, raw-image reads, GPT/MBR discovery, and source-integrity tests.
- **Ext support:** added the read-only Ext2/3/4 backend, metadata/path access,
  image probing, CLI inspection, and image-based validation.
- **Physical discovery:** added Windows physical-disk and volume probing,
  read-only handles, structured scan diagnostics, and `%LOCALAPPDATA%` scan
  logging to diagnose the RAID/sector-size investigation.
- **Mount reliability:** connected the generic filesystem interface to WinFsp,
  fixed browse/unmount lifecycle ownership, preserved read-only denial behavior,
  and verified dynamic drive-letter selection.
- **Desktop polish:** added the app icon, larger default window, About dialog,
  version propagation from `VERSION`, and the polished source-selection UI.
- **Current extension:** routed all consumers through a backend registry and
  added SquashFS plus bounded XFS image support without adding source writes.

## 2026-08-13 — push and filesystem backend extension

### Starting point

- Pushed the current application state to `origin/master` at commit `c2b5a7f`.
- The pushed version is `1.6.1`, including the dynamic free-drive-letter selection.
- The source safety contract remains read-only at the block, filesystem, WinFsp, and UI layers.

### Implementation plan

1. Preserve the existing `ReadOnlyFilesystem` interface and WinFsp adapter.
2. Add a backend registry that probes filesystem signatures and returns a common read-only backend.
3. Add SquashFS using bounded random reads over the existing `BlockReader` abstraction.
4. Add XFS using the maintained pure-Rust reader, with hard 512 MiB image and 64 MiB file materialization ceilings because that reader currently requires whole buffers.
5. Route image opening, physical scanning, CLI inspection, and mounting through the registry.
6. Add focused tests, run the workspace checks, and push the result.

### Walkthrough

The registry checks the source magic first: `hsqs` selects SquashFS, `XFSB`
selects XFS, and other sources continue through the existing Ext2/3/4 probe.
SquashFS keeps the source as a random-access reader and does not load the
physical source wholesale. XFS is deliberately fail-closed above 512 MiB;
this prevents a large physical partition from becoming an unsafe allocation.

All three backends implement only metadata, directory enumeration, file reads,
and symlink reads. The WinFsp mutation callbacks remain denied, and no source
write API is introduced.

### Validation notes

- SquashFS reader API: [`squashfs_reader`](https://docs.rs/squashfs_reader/latest/squashfs_reader/)
- XFS reader API and its whole-image limitation: [`xfs-core`](https://docs.rs/xfs-core/latest/xfs/)
- XFS on-disk format reference: [Linux XFS format header](https://github.com/torvalds/linux/blob/master/fs/xfs/libxfs/xfs_format.h)

The final entry for this change will record the exact test/build output and
commit hash after implementation is verified.

### Verification result

- `cargo fmt --all` — passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace --exclude linuxfs-preview` — passed.
- `cargo build --workspace` — passed.
- `cargo test --workspace` — all tests before the preview binary passed; the
  Windows preview executable was blocked by the embedded `requireAdministrator`
  manifest when launched from this non-elevated shell (OS error 740).

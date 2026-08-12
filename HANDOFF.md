# LinuxFS Manager — Development Handoff

Date: 2026-08-12

## Current state

Image workflows work. The packaged application can:

- open a raw image through the native file picker;
- inspect Ext2/Ext3/Ext4 images;
- list and read files through the CLI;
- mount supported image sources read-only through WinFsp;
- unmount and open the mounted source in Explorer;
- ship with `winfsp-x64.dll` beside `LinuxFSManager.exe`;
- preserve source-image integrity in automated tests.

The current package is:

```text
dist/LinuxFSManager-win64.zip
```

The latest pushed commit is `bd0d014`.

## Unresolved issue

Physical-drive discovery does not find the user’s Ext4 partitions.

The user reports four HDDs. Windows reports:

- Drive 0: GPT, with an unknown partition of about 1 TB;
- Drive 3: GPT, with two Basic partitions, approximately 1.5 TB and 497 GB;
- Drives 1 and 2 are ordinary Windows disks.

The application reports that no compatible Ext filesystem was found on the physical drives. This remains true when the application is launched elevated.

## Work already attempted

1. Added physical-disk enumeration for `PhysicalDrive0..31`.
2. Added read-only Windows raw-device opening using `CreateFileW` with:
   - `GENERIC_READ` only;
   - `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`.
3. Replaced normal file metadata sizing with `IOCTL_DISK_GET_LENGTH_INFO`.
4. Added explicit Administrator elevation to the preview executable manifest.
5. Added a dedicated `Scan Drives` button.
6. Added per-disk and per-partition probe diagnostics instead of silently dropping all errors.
7. Added a fallback scan of Windows volume devices (`\\.\\A:` through `\\.\\Z:`).
8. Added WinFsp runtime preloading from the registered installation path.
9. Packaged `winfsp-x64.dll` beside the executable.

## Important evidence

The development shell itself runs at medium integrity and cannot directly open `\\.\\PhysicalDrive0`; PowerShell reports Access Denied. The application package requests elevation, but the physical Ext4 issue still reproduces when the user says it is run elevated.

Windows Storage Management reports the disks and partitions correctly, so the remaining failure is likely in one of these areas:

- raw-device reads are not returning the expected GPT bytes;
- the GPT parser rejects the real disk layout or geometry;
- `FileExt::seek_read` is incompatible with these raw disk handles;
- the Linux partition starts at an offset/length not being handled correctly;
- the Ext parser rejects the filesystem because of an unsupported feature or filesystem state;
- Windows volume-device probing is not actually opening the Linux volume handles.

## Next debugging work

Do not change the UI first. Add a dedicated elevated diagnostic path that records, for every disk:

1. successful open and reported byte length;
2. a 512-byte read at offset 0;
3. a 512-byte read at offset 512;
4. the first 16 bytes of each sector in hexadecimal;
5. GPT detection and exact GPT parser error;
6. every partition offset and byte length;
7. a read at each partition offset;
8. the Ext superblock magic at partition offset + 1024;
9. the exact `ext4_view` error.

Use a small elevated diagnostic executable or write a diagnostic log to `%LOCALAPPDATA%\\LinuxFS Manager\\scan.log`. Do not log file contents or modify disks.

Compare those bytes with a trusted Windows disk inspection tool or a Linux/libguestfs image of the same disk. The key expected Ext superblock magic is `53 EF` at partition-start + 1024.

If raw-disk reads fail, replace `FileExt::seek_read` with an explicit overlapped/`ReadFile` implementation using aligned buffers and offsets. If GPT parsing fails, fix the parser based on the exact error instead of adding more heuristics. If GPT and reads succeed but the magic is absent, verify that the Windows partition offsets are being interpreted as bytes and that the physical disk’s logical sector size is 512 rather than 4096.

## Safety constraints

- Never open the source with write access.
- Never write partition tables, filesystem metadata, journals, or repair data.
- Never use `diskpart clean`, format, initialize, or other destructive commands.
- Keep image and physical-device source integrity tests.
- Do not claim physical-drive support is complete until Drive 0/3 are verified against the user’s actual disks.

## Verification baseline

Before the next handoff, run:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
cargo build --release -p linuxfs-preview
git diff --check
```

The repository history contains the successive pushed fixes. The latest commit is `bd0d014`; the worktree should remain clean except for this new handoff document until the next debugging session begins.

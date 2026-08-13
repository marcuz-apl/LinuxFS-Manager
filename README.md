# LinuxFS Manager

> **Alfazen Inc.** — A lightweight Windows utility for inspecting Linux filesystems safely.

LinuxFS Manager provides read-only access to Linux filesystem images, partitions, and supported physical-disk sources from Windows. It combines a Rust filesystem core, a Slint desktop interface, and WinFsp mounting so users can inspect Linux data through familiar Windows tools without modifying the source.

## Key capabilities

- **Read-only by design:** Source block devices, filesystem metadata, and mounted views are never opened for writing.
- **Physical-disk discovery:** Finds supported Linux partitions alongside Windows volumes, including hidden data partitions.
- **Raw-image access:** Opens filesystem images through the desktop application or the inspection CLI.
- **Windows integration:** Mounts supported sources as a Windows drive or mount point through WinFsp.
- **Safe inspection:** Uses bounded reads, checked offsets, structured errors, and source-integrity regression tests.
- **Portable diagnostics:** Records physical-scan diagnostics without logging file contents or changing source media.

## Supported filesystems

| Filesystem | Read metadata | Browse directories | Read files | Current limit |
| --- | :---: | :---: | :---: | --- |
| Ext2 / Ext3 / Ext4 | Yes | Yes | Yes | Parser compatibility and source integrity rules apply |
| SquashFS 4.0 | Yes | Yes | Yes | Streamed through bounded random reads |
| XFS | Yes | Yes | Yes | Images up to 2 GiB; individual file reads up to 64 MiB |

Unsupported formats are identified and rejected with an explanatory error. Write operations, filesystem repair, journal replay, partition editing, LVM, LUKS, MD RAID assembly, Btrfs, F2FS, ZFS, and custom kernel drivers are outside the current product scope.

## Read-only safety promise

LinuxFS Manager V1 must never modify the source Linux filesystem. The safety rule is enforced at multiple layers:

1. Physical devices, partitions, and image files are opened with read-only access.
2. Filesystem backends expose inspection and read operations only.
3. WinFsp mutation callbacks return access-denied/read-only results.
4. The UI does not expose create, delete, rename, write, repair, or formatting actions.

Copying files means copying them from the mounted Linux source to a Windows destination; it never copies into the Linux source.

## Architecture

```text
Slint desktop UI
        │
Application and mount services
        │
Read-only storage and BlockReader
        │
Filesystem backend registry
        ├── Ext2/3/4
        ├── SquashFS
        └── XFS
        │
WinFsp read-only adapter
```

The same bounded reader abstraction is used for raw images and physical partitions. Filesystem parsers do not depend on the source type, and the UI does not parse partition tables or filesystem structures.

## Using the application

The packaged Windows executable is:

```text
LinuxFSManager.exe
```

The application starts with a 1200×820 window centered in the primary monitor work area. Use **Scan Drives** to discover physical sources, **Open Image…** to inspect an image, **Mount** to expose a supported source through WinFsp, and **Unmount** to release it.

## Desktop languages

The desktop UI is available in English, Français, Deutsch, Español, Português
(Brasil), Italiano, Polski, Русский, 简体中文, 繁體中文, 日本語, and 한국어. The
header selector defaults to **Automatic (Windows)**, which uses the Windows user
locale when supported and otherwise falls back to English. An explicit choice is
saved in `%APPDATA%\LinuxFS Manager\config.toml` and can be cleared by selecting
Automatic again.

Filesystem labels, file names, paths, UUIDs, drive letters, and raw Windows
errors remain exact source values; the application does not translate them.

Each language is also shipped as its own UTF-8 file in
`locales\<language-tag>.toml` beside the executable. LinuxFS Manager reads the
selected file at startup and when the selector changes. A missing, malformed,
or mismatched file falls back safely to the embedded catalog.

## CLI inspection

The CLI never opens a source for writing and streams regular-file output:

```powershell
cargo run -p linuxfs-cli -- inspect .\disk.img
cargo run -p linuxfs-cli -- ls .\disk.img /
cargo run -p linuxfs-cli -- cat .\disk.img /home/user/readme.txt
```

## Test fixtures

The repository includes a reproducible WSL-based generator for disposable SquashFS and XFS images:

```powershell
.\tools\generate-linux-fixtures.ps1
cargo run -p linuxfs-cli -- inspect .\tests\fixtures-linux\generated\squashfs.img
cargo run -p linuxfs-cli -- inspect .\tests\fixtures-linux\generated\xfs.img
```

The generator requires WSL2 with `mksquashfs` and `mkfs.xfs` (`xfsprogs`). Generated images are ignored by Git and contain no private or source-disk data.

## Prerequisites and packaging

- Windows 10/11 x64 for physical-device access and desktop mounting.
- Rust 1.97 or newer for development.
- WinFsp installed for mounting. At startup, the app checks its registered installation, architecture-matched runtime DLL, `WinFsp.Launcher` service, and runtime initialization before it creates any mount service. If a check fails, the app provides an official-download link and a **Recheck** action; it never downloads, installs, or starts WinFsp itself.
- The portable package includes the reviewed `winfsp-x64.dll` user-mode runtime asset, but that DLL alone cannot provide WinFsp’s Windows driver/framework. Each startup assessment is recorded only for diagnostics in `%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml`; the record can never authorize a mount.

Build a verified portable package with:

```powershell
.\tools\package-release.ps1 -Tag portable -WinFspDll C:\path\to\winfsp-x64.dll
```

See [docs/packaging.md](docs/packaging.md) for the prerequisite and redistribution contract.

## Development checks

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --exclude linuxfs-preview
cargo build --workspace
cargo build --release --workspace
```

The elevated preview executable may require an Administrator shell to execute its Windows integration tests because its manifest requests elevation.

## Documentation

- [Product requirements](PRD.md)
- [Development and safety rules](AGENTS.md)
- [Development log](docs/Dev-logs.md)
- [Packaging and prerequisites](docs/packaging.md)
- [Current handoff](HANDOFF.md)

The current version is tracked in [`VERSION`](VERSION), and the repository hooks apply the project’s bounded `m.n.p` versioning rules to commits.

## License

Copyright © 2026 Alfazen Inc. LinuxFS Manager is free software licensed under the [GNU General Public License, version 3 or later](LICENSE). Source and binary recipients may copy, modify, and redistribute it under those terms.

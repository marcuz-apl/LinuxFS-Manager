# V1 packaging and prerequisites

## Product executable

The packaged application executable is:

```text
LinuxFSManager.exe
```

Release artifacts should target Windows 10/11 x64. The repository name remains
`linuxfs-manager`.

## WinFsp prerequisite

WinFsp is required before LinuxFS Manager creates its mount service. On every
startup, the application performs a fresh, read-only assessment of the
registered WinFsp installation, architecture-matched runtime DLL,
`WinFsp.Launcher` service, and runtime initialization. If any check fails, the
application opens a prerequisite screen instead of the main application and
directs the user to install the supported WinFsp release from the official
source:

<https://winfsp.dev/rel/>

The prerequisite screen supplies **Download WinFsp** and **Recheck** actions.
It does not download, install, start, stop, or otherwise modify WinFsp or any
Windows service. The user runs the official MSI and accepts its driver
installation, then asks the app to recheck. Detection must not be represented
as a successful mount.

Every live assessment is recorded atomically at
`%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml`. This small TOML record
contains diagnostic state only and is never trusted to authorize a mount or
bypass a fresh live check. The application continues to enforce read-only
behavior after the prerequisite is installed.

## Distribution modes

The preferred user distribution is a signed Windows installer containing
`LinuxFSManager.exe` and the reviewed WinFsp installation/remediation flow.

An optional portable ZIP may contain the application and its user-mode assets,
but it cannot claim to be self-contained: WinFsp is an external Windows
driver/framework prerequisite and may need a separate installation. A portable
bundle must detect the missing prerequisite and provide the same remediation
guidance rather than silently failing.

Every portable package must place `winfsp-x64.dll` and the GPL `LICENSE` beside
`LinuxFSManager.exe`. Use the repository script to build and verify the bundle:

```powershell
.\tools\package-release.ps1 -Tag portable -WinFspDll C:\path\to\winfsp-x64.dll
```

The script also searches the registered WinFsp installation and verifies that
both the folder and ZIP contain the DLL. The DLL is a user-mode runtime asset;
the WinFsp driver/framework prerequisite is still required separately.

## License and third-party notices

LinuxFS Manager is distributed under `GPL-3.0-or-later`. The source repository
and every portable package include the full `LICENSE` text and `NOTICE.md`.
Maintain the WinFsp license and notice materials when its installer or other
WinFsp files are distributed. This project does not rebrand or modify WinFsp.

Do not bundle unreviewed WinFsp binaries or describe the application as fully
standalone until WinFsp redistribution terms, Slint licensing, signing, and
the selected installer have been reviewed. Packaging must not add a setting or
startup option that weakens the V1 read-only guarantee.

## Current repository status

Packaging metadata and installer automation are not yet the production release
pipeline. This document records the V1 contract; the final release process must
also validate the executable name, WinFsp prerequisite behavior, clean
unmount-on-exit behavior, and Windows x64 artifacts.

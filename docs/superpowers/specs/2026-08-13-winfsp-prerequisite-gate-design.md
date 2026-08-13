# WinFsp Prerequisite Gate Design

## Goal

Ensure LinuxFS Manager does not present its mounting workflow as usable unless the required WinFsp framework is installed and operational. A portable package may carry `winfsp-x64.dll`, but it must still verify the installed WinFsp driver/framework on the target computer.

## Scope

This change adds a Windows startup gate, status recording, and a user-driven remediation path. It does not download an MSI, run `msiexec`, install a driver, start or stop services, or weaken the read-only filesystem rules.

## Current evidence

On the development computer, the installed WinFsp registry key points to `C:\Program Files (x86)\WinFsp`, and the `WinFsp.Launcher` Windows service is running. These checks confirm the local framework installation, but they must be repeated on every target computer.

## Design

### Live prerequisite assessment

`linuxfs-winfsp` will expose a structured, read-only assessment with these independent checks:

1. A registered WinFsp installation directory is present.
2. The architecture-matching runtime DLL exists at the registered location.
3. The `WinFsp.Launcher` Windows service is installed and running.
4. The runtime DLL can be loaded and `winfsp_init` succeeds.

The assessment is `Ready` only if all four checks pass. It reports a stable diagnostic for the first failed condition and never attempts to repair Windows state.

### Startup behavior

The Windows desktop entry point performs the assessment before constructing mount services. If the assessment is ready, normal startup continues.

If the assessment is not ready, the application shows a prerequisite dialog instead of the main filesystem UI. The dialog explains that mounting requires the WinFsp driver/framework, provides a **Download WinFsp** button that opens the official WinFsp download page, and provides a **Recheck** button. Recheck performs a new live assessment. The user installs WinFsp through its official MSI and UAC prompt, then returns to recheck or relaunch the application.

### Status record

After each assessment, the app writes `%LOCALAPPDATA%\LinuxFS Manager\winfsp-status.toml` atomically. The record contains:

```toml
status_version = 1
checked_at_utc = "2026-08-13T10:55:00Z"
status = "ready"
reason = ""
runtime_path = "C:\\Program Files (x86)\\WinFsp\\bin\\winfsp-x64.dll"
launcher_service = "running"
```

The file contains no filesystem data, credentials, or installer state. It is an audit/troubleshooting record only; it is never trusted to authorize mounting and does not replace the live Windows checks.

### Packaging and installer boundary

The portable ZIP continues to include `winfsp-x64.dll` beside `LinuxFSManager.exe`. This DLL is a process runtime asset, not the WinFsp kernel driver/framework. A future signed bootstrap installer may explicitly install the official WinFsp MSI and then launch the application, subject to WinFsp redistribution/licensing review. That installer is outside this scoped change.

## Error handling

- Missing registry entry, DLL, service, or runtime initialization produces `WinFspUnavailable`.
- The app does not crash, mount, or claim readiness when a check fails.
- Failure to write the status record is logged/visible as a non-authorizing diagnostic; the live prerequisite result still controls application behavior.
- Download-site launch failures are shown in the prerequisite dialog.

## Testing

- Unit tests cover each structured assessment outcome using injected probe results.
- Unit tests confirm status records use TOML and do not control readiness.
- Windows checks remain read-only: registry queries, service status queries, DLL loading, and runtime initialization only.
- The desktop startup branch is checked for both ready and unavailable outcomes; elevated launch requirements are documented separately.

## Safety and security constraints

- Never download or install a driver silently.
- Never invoke `msiexec` or modify a Windows service from the application.
- Never bypass a failed live prerequisite check because of a prior status file.
- Keep every Linux source filesystem operation read-only.

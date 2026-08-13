# LinuxFS Manager Mount Lifecycle and Browse Reliability

## Goal

Make a mounted Ext2/Ext3/Ext4 source consistently browsable through its WinFsp drive letter and make `Unmount` reliably release that drive letter so another source can be mounted.

The V1 read-only guarantee remains unchanged: no source filesystem or image writes are introduced.

## Root cause found

The low-level WinFsp path was reproduced with the generated Ext4 image in an elevated session: directory enumeration, regular-file reads, and teardown all worked. The application path has two lifecycle gaps:

1. The UI keeps a stale copy of the source list row. After a mount, selecting that row can reset the UI to the pre-mount state and hide `Unmount` even though the mount service still owns the drive.
2. The mount service removes its ownership record immediately after the host unmount call and does not verify that the drive letter has actually disappeared. A failed or incomplete release therefore cannot be retried through the application.

## Design

### Source state synchronization

- Treat the selected source and its corresponding source-list row as one logical source.
- On successful mount, update the current source and matching list row with the mount point and mounted status.
- On successful unmount, clear the mount point and restore compatible status in both locations.
- When selecting a source, derive Mount/Unmount/Open Explorer capability from that source’s actual status rather than assuming every discovered row is unmounted.
- While a mount or unmount operation is pending, disable conflicting actions.
- On failure, restore the action that is safe to retry and preserve the mounted state when unmount failed.

### Mount service ownership

- Keep the mount manager in the service map until unmount completes successfully.
- Have the Windows host verify that the configured drive letter is released before returning success.
- If release verification fails, return a WinFsp failure and retain the manager for a retry.
- Reject a new mount when the configured drive letter is already occupied.

### Testing

- Add pure application tests covering mount/unmount state propagation to the selected source and source list.
- Add lifecycle tests proving a failed unmount retains service ownership and a successful unmount removes it.
- Preserve existing read-only filesystem tests and the image source-integrity checks.
- Run formatting, workspace tests (excluding the elevation-only preview runtime test where required), clippy, and a release build.
- Run the WinFsp image smoke test and verify that `L:` is present while mounted and absent after teardown.

## Non-goals

- No write support, journal replay, repair, or source mutation.
- No change to the physical-disk discovery logic.
- No redesign of the filesystem parser or replacement of WinFsp.
- No database or persistent mount-state storage.

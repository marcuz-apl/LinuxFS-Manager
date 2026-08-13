# Physical Scan Diagnostics Design

## Goal

Make physical-drive discovery explain where scanning fails without changing the
read-only storage boundary.

## Design

`linuxfs-windows` will expose a scan report containing discovered Ext
partitions and one diagnostic text entry for every attempted physical drive.
Each opened drive will report its size, bytes at offsets 0 and 512, partition
layout, each partition's byte range, the Ext superblock magic location, and the
exact probe error or success. Open failures will also be retained instead of
being discarded.

`linuxfs-app` will consume the report, preserve the existing source list when
partitions are found, and return a detailed error containing the report when no
partitions are found. The Windows preview will write that report to
`%LOCALAPPDATA%\\LinuxFS Manager\\scan.log` and display it in the details area.
The log contains bounded metadata and signatures only; it never copies file
contents or writes to a source disk.

## Testing

Unit tests will verify report formatting and that scan diagnostics preserve
individual drive failures. Existing image source-integrity and read-only tests
remain unchanged. The packaged Windows executable will be rebuilt with the
WinFsp runtime beside it.

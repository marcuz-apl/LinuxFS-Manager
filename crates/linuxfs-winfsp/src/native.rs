//! Native WinFsp configuration and callback adapter.

use std::{
    ffi::c_void,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

use linuxfs_core::{DirectoryEntry, FileKind, FsPath, NodeMetadata, ReadOnlyFilesystem};
use windows_sys::Win32::Storage::FileSystem::GetLogicalDrives;
use winfsp::{
    FspError, U16CStr,
    filesystem::{
        DirInfo, DirMarker, FileInfo, FileSecurity, FileSystemContext, ModificationDescriptor,
        OpenFileInfo, VolumeInfo, WideNameInfo,
    },
    host::{FileSystemHost, VolumeParams},
};
use winfsp_sys::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES};

use crate::{MountHost, ReadOnlyDispatcher};

const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_NO_MORE_FILES: u32 = 18;
const ERROR_INVALID_PARAMETER: u32 = 87;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const DRIVE_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);
static CALLBACK_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn drive_letter_mask(mount_point: &str) -> linuxfs_core::Result<u32> {
    let mount_point = mount_point.strip_prefix(r"\\.\").unwrap_or(mount_point);
    let bytes = mount_point.as_bytes();
    if bytes.len() != 2 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return Err(linuxfs_core::Error::new(
            linuxfs_core::ErrorCategory::MountPointUnavailable,
            format!("invalid drive-letter mount point {mount_point:?}"),
        ));
    }
    Ok(1_u32 << (bytes[0].to_ascii_uppercase() - b'A'))
}

fn drive_letter_is_present(mask: u32, mount_point: &str) -> bool {
    drive_letter_mask(mount_point)
        .map(|drive_mask| mask & drive_mask != 0)
        .unwrap_or(false)
}

fn select_drive_letter_from_mask(
    mask: u32,
    preferred: Option<&str>,
) -> linuxfs_core::Result<String> {
    if let Some(preferred) = preferred {
        let preferred_mask = drive_letter_mask(preferred)?;
        if mask & preferred_mask == 0 {
            return Ok(preferred
                .strip_prefix(r"\\.\")
                .unwrap_or(preferred)
                .to_ascii_uppercase());
        }
    }

    for index in (0_u8..26).rev() {
        let drive_mask = 1_u32 << index;
        if mask & drive_mask == 0 {
            return Ok(format!("{}:", char::from(b'A' + index)));
        }
    }

    Err(linuxfs_core::Error::new(
        linuxfs_core::ErrorCategory::MountPointUnavailable,
        "all Windows drive letters are currently occupied",
    ))
}

pub fn select_available_drive_letter(preferred: Option<&str>) -> linuxfs_core::Result<String> {
    select_drive_letter_from_mask(logical_drives(), preferred)
}

#[allow(unsafe_code)]
fn logical_drives() -> u32 {
    // SAFETY: GetLogicalDrives has no pointer arguments and returns the current
    // process-wide logical-drive bit mask.
    unsafe { GetLogicalDrives() }
}

fn wait_for_drive_release(mount_point: &str) -> linuxfs_core::Result<()> {
    let deadline = Instant::now() + DRIVE_RELEASE_TIMEOUT;
    loop {
        if !drive_letter_is_present(logical_drives(), mount_point) {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::WinFspFailure,
                format!("WinFsp did not release drive {mount_point} within 2 seconds"),
            ));
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn callback_diagnostic(operation: &str, detail: &str) {
    if std::env::var_os("LINUXFS_MOUNT_DIAGNOSTICS").is_none() {
        return;
    }
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let directory = std::path::PathBuf::from(local_app_data).join("LinuxFS Manager");
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join("mount.log");
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let sequence = CALLBACK_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let _ = writeln!(file, "{sequence}: {operation}: {detail}");
}

pub fn read_only_volume_params(filesystem_name: &str) -> VolumeParams {
    let mut params = VolumeParams::new();
    params
        .filesystem_name(filesystem_name)
        .read_only_volume(true)
        .case_preserved_names(true)
        .unicode_on_disk(true)
        .persistent_acls(false)
        .named_streams(false)
        .extended_attributes(false)
        .reparse_points(false);
    params
}

pub fn deny_mutation() -> winfsp::Result<()> {
    Err(FspError::WIN32(ERROR_ACCESS_DENIED))
}

#[derive(Debug)]
pub struct OpenHandle {
    pub(crate) path: FsPath,
    pub(crate) metadata: NodeMetadata,
}

pub struct ReadOnlyContext<F> {
    dispatcher: ReadOnlyDispatcher<F>,
}

impl<F> ReadOnlyContext<F>
where
    F: ReadOnlyFilesystem,
{
    pub fn new(filesystem: F) -> Self {
        Self {
            dispatcher: ReadOnlyDispatcher::new(filesystem),
        }
    }

    fn path(file_name: &U16CStr) -> winfsp::Result<FsPath> {
        let path = file_name.to_string_lossy().replace('\\', "/");
        let path = if path.starts_with('/') {
            path
        } else {
            format!("/{path}")
        };
        FsPath::parse(&path).map_err(map_error)
    }

    fn attributes(metadata: NodeMetadata) -> u32 {
        match metadata.kind {
            FileKind::Directory => FILE_ATTRIBUTE_DIRECTORY,
            FileKind::Symlink => FILE_ATTRIBUTE_REPARSE_POINT,
            _ => FILE_ATTRIBUTE_NORMAL,
        }
    }

    fn fill_file_info(metadata: NodeMetadata, file_info: &mut FileInfo) {
        file_info.file_attributes = Self::attributes(metadata);
        file_info.file_size = metadata.size;
        file_info.allocation_size = metadata.size;
    }

    fn directory_entries(&self, path: &FsPath) -> linuxfs_core::Result<Vec<DirectoryEntry>> {
        Ok(self
            .dispatcher
            .read_dir(path)?
            .into_iter()
            .filter(|entry| entry.name != "." && entry.name != "..")
            .collect())
    }
}

impl<F> FileSystemContext for ReadOnlyContext<F>
where
    F: ReadOnlyFilesystem + Send + Sync,
{
    type FileContext = OpenHandle;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [c_void]>,
        _reparse_point_resolver: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> winfsp::Result<FileSecurity> {
        callback_diagnostic("get_security.begin", "request received");
        let path = Self::path(file_name)?;
        let metadata = self.dispatcher.lookup(&path).map_err(map_error)?;
        callback_diagnostic(
            "get_security.end",
            if metadata.kind == FileKind::Directory {
                "directory"
            } else {
                "non-directory"
            },
        );
        Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: Self::attributes(metadata),
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: FILE_ACCESS_RIGHTS,
        file_info: &mut OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        callback_diagnostic("open.begin", "request received");
        let path = Self::path(file_name)?;
        let metadata = self.dispatcher.lookup(&path).map_err(map_error)?;
        Self::fill_file_info(metadata, file_info.as_mut());
        callback_diagnostic(
            "open.end",
            if metadata.kind == FileKind::Directory {
                "directory"
            } else {
                "non-directory"
            },
        );
        Ok(OpenHandle { path, metadata })
    }

    fn close(&self, _context: Self::FileContext) {}

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        Self::fill_file_info(context.metadata, file_info);
        Ok(())
    }

    fn read(
        &self,
        context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> winfsp::Result<u32> {
        callback_diagnostic("read.begin", "request received");
        let count = self
            .dispatcher
            .read_file_at(&context.path, offset, buffer)
            .map_err(map_error)?;
        callback_diagnostic("read.end", "request completed");
        u32::try_from(count).map_err(|_| FspError::WIN32(ERROR_INVALID_PARAMETER))
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker,
        buffer: &mut [u8],
    ) -> winfsp::Result<u32> {
        let marker_detail = marker
            .inner_as_cstr()
            .map(|value| value.to_string_lossy())
            .unwrap_or_else(|| "<none>".to_owned());
        callback_diagnostic("read_directory.begin", &format!("marker={marker_detail}"));
        let marker = marker.inner().map(String::from_utf16_lossy);
        let mut entries = self.directory_entries(&context.path).map_err(map_error)?;
        entries.sort_by_cached_key(|entry| entry.name.encode_utf16().collect::<Vec<_>>());
        callback_diagnostic(
            "read_directory.entries",
            &format!("count={}", entries.len()),
        );

        let mut cursor = 0_u32;
        for entry in entries {
            if marker.as_deref().is_some_and(|marker| {
                entry.name.encode_utf16().collect::<Vec<_>>()
                    <= marker.encode_utf16().collect::<Vec<_>>()
            }) {
                continue;
            }
            let mut info = DirInfo::<255>::new();
            info.set_name(&entry.name)?;
            Self::fill_file_info(entry.metadata, info.file_info_mut());
            if !info.append_to_buffer(buffer, &mut cursor) {
                break;
            }
        }
        let _ = DirInfo::<255>::finalize_buffer(buffer, &mut cursor);
        let count = cursor;
        callback_diagnostic("read_directory.end", &format!("bytes={count}"));
        if count == 0 {
            return Err(FspError::WIN32(ERROR_NO_MORE_FILES));
        }
        Ok(count)
    }

    fn get_volume_info(&self, out_volume_info: &mut VolumeInfo) -> winfsp::Result<()> {
        let info = self.dispatcher.info().map_err(map_error)?;
        out_volume_info.total_size = info.total_size.unwrap_or(0);
        out_volume_info.free_size = info.free_size.unwrap_or(0);
        out_volume_info.set_volume_label(
            info.label
                .as_deref()
                .unwrap_or(info.filesystem_type.as_str()),
        );
        Ok(())
    }

    fn write(
        &self,
        _context: &Self::FileContext,
        _buffer: &[u8],
        _offset: u64,
        _write_to_eof: bool,
        _constrained_io: bool,
        _file_info: &mut FileInfo,
    ) -> winfsp::Result<u32> {
        Err(FspError::WIN32(ERROR_ACCESS_DENIED))
    }

    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        _file_name: &U16CStr,
        _create_options: u32,
        _granted_access: FILE_ACCESS_RIGHTS,
        _file_attributes: FILE_FLAGS_AND_ATTRIBUTES,
        _security_descriptor: Option<&[c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        _file_info: &mut OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        Err(FspError::WIN32(ERROR_ACCESS_DENIED))
    }

    fn flush(
        &self,
        _context: Option<&Self::FileContext>,
        _file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    fn set_security(
        &self,
        _context: &Self::FileContext,
        _security_information: u32,
        _modification_descriptor: ModificationDescriptor,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    fn rename(
        &self,
        _context: &Self::FileContext,
        _file_name: &U16CStr,
        _new_file_name: &U16CStr,
        _replace_if_exists: bool,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    #[allow(clippy::too_many_arguments)]
    fn set_basic_info(
        &self,
        _context: &Self::FileContext,
        _file_attributes: u32,
        _creation_time: u64,
        _last_access_time: u64,
        _last_write_time: u64,
        _last_change_time: u64,
        _file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    fn set_delete(
        &self,
        _context: &Self::FileContext,
        _file_name: &U16CStr,
        _delete_file: bool,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    fn set_file_size(
        &self,
        _context: &Self::FileContext,
        _new_size: u64,
        _set_allocation_size: bool,
        _file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }

    fn set_volume_label(
        &self,
        _volume_label: &U16CStr,
        _volume_info: &mut VolumeInfo,
    ) -> winfsp::Result<()> {
        deny_mutation()
    }
}

pub fn new_host<F>(
    filesystem: F,
    filesystem_name: &str,
) -> winfsp::Result<FileSystemHost<ReadOnlyContext<F>>>
where
    F: ReadOnlyFilesystem + Send + Sync,
{
    Ok(FileSystemHost::new(
        read_only_volume_params(filesystem_name),
        ReadOnlyContext::new(filesystem),
    )?)
}

/// Native host implementation used by the platform-independent mount manager.
pub struct NativeMountHost<F>
where
    F: ReadOnlyFilesystem + Send + Sync,
{
    host: FileSystemHost<ReadOnlyContext<F>>,
    mount_point: String,
    started: bool,
}

impl<F> NativeMountHost<F>
where
    F: ReadOnlyFilesystem + Send + Sync,
{
    pub fn new(
        filesystem: F,
        filesystem_name: &str,
        mount_point: impl Into<String>,
    ) -> winfsp::Result<Self> {
        Ok(Self {
            host: new_host(filesystem, filesystem_name)?,
            mount_point: mount_point.into(),
            started: false,
        })
    }
}

impl<F> MountHost for NativeMountHost<F>
where
    F: ReadOnlyFilesystem + Send + Sync,
{
    fn mount(&mut self) -> linuxfs_core::Result<()> {
        if self.started {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::WinFspFailure,
                "WinFsp host is already started",
            ));
        }
        if drive_letter_is_present(logical_drives(), &self.mount_point) {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::MountPointUnavailable,
                format!("drive {} is already in use", self.mount_point),
            ));
        }
        self.host
            .start()
            .map_err(|error| map_host_error(error.into()))?;
        self.started = true;
        if let Err(error) = self.host.mount(&self.mount_point) {
            self.host.stop();
            self.started = false;
            return Err(map_host_error(error.into()));
        }
        Ok(())
    }

    fn unmount(&mut self) -> linuxfs_core::Result<()> {
        if !self.started {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::WinFspFailure,
                "WinFsp host is not started",
            ));
        }
        self.host.unmount();
        self.host.stop();
        wait_for_drive_release(&self.mount_point)?;
        self.started = false;
        Ok(())
    }
}

fn map_host_error(error: FspError) -> linuxfs_core::Error {
    linuxfs_core::Error::with_source(
        linuxfs_core::ErrorCategory::WinFspFailure,
        "WinFsp mount operation failed",
        error,
    )
}
fn map_error(error: linuxfs_core::Error) -> FspError {
    match error.category() {
        linuxfs_core::ErrorCategory::PermissionDenied => FspError::WIN32(ERROR_ACCESS_DENIED),
        linuxfs_core::ErrorCategory::StorageAccess => FspError::WIN32(ERROR_FILE_NOT_FOUND),
        linuxfs_core::ErrorCategory::InvalidImage => FspError::WIN32(ERROR_INVALID_PARAMETER),
        _ => FspError::WIN32(ERROR_INVALID_PARAMETER),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFilesystem;

    impl ReadOnlyFilesystem for FakeFilesystem {
        fn info(&self) -> linuxfs_core::Result<linuxfs_core::FilesystemInfo> {
            Ok(linuxfs_core::FilesystemInfo {
                filesystem_type: "ext4".to_owned(),
                label: Some("TEST".to_owned()),
                uuid: None,
                block_size: None,
                total_size: None,
                free_size: None,
            })
        }

        fn lookup(&self, _path: &FsPath) -> linuxfs_core::Result<NodeMetadata> {
            Ok(NodeMetadata {
                kind: FileKind::Directory,
                size: 0,
                permissions: 0o755,
                uid: 0,
                gid: 0,
            })
        }

        fn read_dir(
            &self,
            _path: &FsPath,
        ) -> linuxfs_core::Result<Vec<linuxfs_core::DirectoryEntry>> {
            Ok(vec![linuxfs_core::DirectoryEntry {
                name: "hello.txt".to_owned(),
                metadata: NodeMetadata {
                    kind: FileKind::Regular,
                    size: 5,
                    permissions: 0o644,
                    uid: 1000,
                    gid: 1000,
                },
            }])
        }

        fn read_file_at(
            &self,
            _path: &FsPath,
            _offset: u64,
            _destination: &mut [u8],
        ) -> linuxfs_core::Result<usize> {
            Ok(0)
        }

        fn read_link(&self, _path: &FsPath) -> linuxfs_core::Result<FsPath> {
            Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::UnsupportedFilesystem,
                "not a symlink",
            ))
        }
    }

    #[test]
    fn directory_listing_uses_filesystem_navigation_entries() {
        let context = ReadOnlyContext::new(FakeFilesystem);
        let entries = context
            .directory_entries(&FsPath::root())
            .expect("directory listing");
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["hello.txt"]
        );
    }

    #[test]
    fn native_configuration_is_constructible() {
        let _params = read_only_volume_params("LinuxFS Manager");
        assert!(deny_mutation().is_err());
    }

    #[test]
    fn drive_letter_helpers_validate_and_match_masks() {
        assert_eq!(
            drive_letter_mask("L:").expect("valid drive letter"),
            1 << 11
        );
        assert_eq!(
            drive_letter_mask(r"\\.\L:").expect("global drive letter"),
            1 << 11
        );
        assert_eq!(
            drive_letter_mask("l:").expect("valid drive letter"),
            1 << 11
        );
        assert!(drive_letter_mask("L:\\").is_err());
        assert!(drive_letter_mask("volume").is_err());
        assert!(drive_letter_is_present(1 << 11, "L:"));
        assert!(!drive_letter_is_present(1 << 10, "L:"));
    }

    #[test]
    fn drive_letter_selection_prefers_configured_free_letter() {
        assert_eq!(
            select_drive_letter_from_mask(1 << 25, Some("L:")).expect("free preferred letter"),
            "L:"
        );
    }

    #[test]
    fn drive_letter_selection_falls_back_to_highest_free_letter() {
        assert_eq!(
            select_drive_letter_from_mask((1 << 25) | (1 << 24) | (1 << 11), Some("L:"))
                .expect("free fallback"),
            "X:"
        );
    }

    #[test]
    fn drive_letter_selection_rejects_when_all_letters_are_used() {
        let error = select_drive_letter_from_mask(u32::MAX, None)
            .expect_err("all drive letters are occupied");
        assert_eq!(
            error.category(),
            linuxfs_core::ErrorCategory::MountPointUnavailable
        );
    }
}

//! Native WinFsp configuration and callback adapter.

use std::ffi::c_void;

use linuxfs_core::{FileKind, FsPath, NodeMetadata, ReadOnlyFilesystem};
use winfsp::{
    FspError, U16CStr,
    filesystem::{
        FileInfo, FileSecurity, FileSystemContext, ModificationDescriptor, OpenFileInfo, VolumeInfo,
    },
    host::{FileSystemHost, VolumeParams},
};
use winfsp_sys::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES};

use crate::{MountHost, ReadOnlyDispatcher};

const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_INVALID_PARAMETER: u32 = 87;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

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

#[derive(Debug, Clone)]
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
        let path = Self::path(file_name)?;
        let metadata = self.dispatcher.lookup(&path).map_err(map_error)?;
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
        let path = Self::path(file_name)?;
        let metadata = self.dispatcher.lookup(&path).map_err(map_error)?;
        Self::fill_file_info(metadata, file_info.as_mut());
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
        let count = self
            .dispatcher
            .read_file_at(&context.path, offset, buffer)
            .map_err(map_error)?;
        u32::try_from(count).map_err(|_| FspError::WIN32(ERROR_INVALID_PARAMETER))
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

    #[test]
    fn native_configuration_is_constructible() {
        let _params = read_only_volume_params("LinuxFS Manager");
        assert!(deny_mutation().is_err());
    }
}

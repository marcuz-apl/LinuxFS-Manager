#[cfg(windows)]
pub mod native;

use linuxfs_core::{
    DirectoryEntry, FilesystemInfo, FsPath, NodeMetadata, ReadOnlyFilesystem, Result,
};
use std::path::PathBuf;

/// Result of checking whether the WinFsp runtime can be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinFspStatus {
    /// The WinFsp runtime is available to this process.
    Available,
    /// WinFsp is not available, with a stable diagnostic reason.
    Unavailable { reason: WinFspUnavailableReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinFspUnavailableReason {
    /// This diagnostic was called on a non-Windows target.
    UnsupportedPlatform,
    /// The WinFsp runtime could not be loaded.
    RuntimeUnavailable,
}

impl WinFspStatus {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

/// Checks WinFsp availability without installing, starting, stopping, or changing anything.
pub fn check_winfsp() -> WinFspStatus {
    platform::check()
}

/// Loads the installed WinFsp runtime from its absolute path before any
/// delay-loaded WinFsp API is called.
pub fn prepare_runtime() -> Result<()> {
    platform::prepare_runtime()
}

/// Returns the registered WinFsp installation directory without changing system state.
pub fn winfsp_installation_dir() -> Option<PathBuf> {
    platform::installation_dir()
}

/// Returns the registered architecture-specific WinFsp runtime DLL path.
pub fn winfsp_runtime_path() -> Option<PathBuf> {
    let directory = winfsp_installation_dir()?;
    Some(directory.join("bin").join(runtime_dll_name()))
}

fn runtime_dll_name() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "winfsp-x64.dll"
    } else if cfg!(target_arch = "x86") {
        "winfsp-x86.dll"
    } else if cfg!(target_arch = "aarch64") {
        "winfsp-a64.dll"
    } else {
        "winfsp-unknown.dll"
    }
}

#[cfg(windows)]
mod platform {
    use super::{WinFspStatus, WinFspUnavailableReason};
    use std::path::PathBuf;

    pub fn prepare_runtime() -> linuxfs_core::Result<()> {
        super::load_runtime_dll()
    }

    pub fn check() -> WinFspStatus {
        if super::load_runtime_dll().is_err() {
            return WinFspStatus::Unavailable {
                reason: WinFspUnavailableReason::RuntimeUnavailable,
            };
        }
        match winfsp::winfsp_init() {
            Ok(_init) => WinFspStatus::Available,
            Err(_) => WinFspStatus::Unavailable {
                reason: WinFspUnavailableReason::RuntimeUnavailable,
            },
        }
    }

    pub fn installation_dir() -> Option<PathBuf> {
        super::registry_installation_dir()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{WinFspStatus, WinFspUnavailableReason};
    use std::path::PathBuf;

    pub fn check() -> WinFspStatus {
        WinFspStatus::Unavailable {
            reason: WinFspUnavailableReason::UnsupportedPlatform,
        }
    }

    pub fn prepare_runtime() -> linuxfs_core::Result<()> {
        Ok(())
    }

    pub fn installation_dir() -> Option<PathBuf> {
        None
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn load_runtime_dll() -> linuxfs_core::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::LibraryLoader::LoadLibraryW;

    let path = winfsp_runtime_path().ok_or_else(|| {
        linuxfs_core::Error::new(
            linuxfs_core::ErrorCategory::WinFspUnavailable,
            "WinFsp installation was not found in the Windows registry",
        )
    })?;
    if !path.is_file() {
        return Err(linuxfs_core::Error::new(
            linuxfs_core::ErrorCategory::WinFspUnavailable,
            format!("WinFsp runtime DLL was not found at {}", path.display()),
        ));
    }
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a valid, NUL-terminated UTF-16 path buffer. The
    // returned module is intentionally kept loaded for the process lifetime.
    let module = unsafe { LoadLibraryW(wide.as_ptr()) };
    if module.is_null() {
        return Err(linuxfs_core::Error::new(
            linuxfs_core::ErrorCategory::WinFspUnavailable,
            format!(
                "Windows could not load WinFsp runtime at {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn load_runtime_dll() -> linuxfs_core::Result<()> {
    Ok(())
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn registry_installation_dir() -> Option<PathBuf> {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};
    use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};

    for key in ["SOFTWARE\\WOW6432Node\\WinFsp", "SOFTWARE\\WinFsp"] {
        let key: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
        let value: Vec<u16> = "InstallDir"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let mut buffer = [0u16; 260];
        let mut size = (buffer.len() * std::mem::size_of::<u16>()) as u32;
        // SAFETY: all pointers reference owned, NUL-terminated buffers and the output buffer
        // is accompanied by its byte capacity; the call only reads registry state.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status == 0 && size >= 2 {
            let units = (size as usize / 2).saturating_sub(1);
            let path = OsString::from_wide(&buffer[..units]);
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn registry_installation_dir() -> Option<PathBuf> {
    None
}

/// Windows filesystem requests handled by the adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinOperation {
    QueryVolumeInformation,
    Open,
    QueryFileInformation,
    ReadDirectory,
    Read,
    ReadLink,
    Cleanup,
    Close,
    Create,
    Write,
    SetFileSize,
    CanDelete,
    Delete,
    Rename,
    SetBasicInformation,
    SetSecurity,
    Flush,
}

/// The only decisions permitted at the V1 WinFsp boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationDecision {
    AllowRead,
    DenyReadOnly,
}

impl WinOperation {
    pub fn decision(self) -> OperationDecision {
        match self {
            Self::QueryVolumeInformation
            | Self::Open
            | Self::QueryFileInformation
            | Self::ReadDirectory
            | Self::Read
            | Self::ReadLink
            | Self::Cleanup
            | Self::Close => OperationDecision::AllowRead,
            Self::Create
            | Self::Write
            | Self::SetFileSize
            | Self::CanDelete
            | Self::Delete
            | Self::Rename
            | Self::SetBasicInformation
            | Self::SetSecurity
            | Self::Flush => OperationDecision::DenyReadOnly,
        }
    }
}

/// Read-only filesystem operations exposed to a future native WinFsp host.
///
/// The native binding must call `authorize` before handling every request.
pub struct ReadOnlyDispatcher<F> {
    filesystem: F,
}

impl<F> ReadOnlyDispatcher<F>
where
    F: ReadOnlyFilesystem,
{
    pub fn new(filesystem: F) -> Self {
        Self { filesystem }
    }

    pub fn authorize(operation: WinOperation) -> Result<()> {
        match operation.decision() {
            OperationDecision::AllowRead => Ok(()),
            OperationDecision::DenyReadOnly => Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::PermissionDenied,
                "operation denied: LinuxFS Manager is read-only",
            )),
        }
    }

    pub fn info(&self) -> Result<FilesystemInfo> {
        self.filesystem.info()
    }

    pub fn lookup(&self, path: &FsPath) -> Result<NodeMetadata> {
        self.filesystem.lookup(path)
    }

    pub fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>> {
        self.filesystem.read_dir(path)
    }

    pub fn read_file_at(
        &self,
        path: &FsPath,
        offset: u64,
        destination: &mut [u8],
    ) -> Result<usize> {
        self.filesystem.read_file_at(path, offset, destination)
    }

    pub fn read_link(&self, path: &FsPath) -> Result<FsPath> {
        self.filesystem.read_link(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linuxfs_core::{ErrorCategory, FileKind};

    #[cfg(not(windows))]
    #[test]
    fn diagnostic_is_structured_and_read_only_on_this_platform() {
        let status = check_winfsp();
        assert!(!status.is_available());
        assert_eq!(
            status,
            WinFspStatus::Unavailable {
                reason: WinFspUnavailableReason::UnsupportedPlatform
            }
        );
    }

    #[cfg(windows)]
    #[test]
    fn diagnostic_returns_a_known_windows_status() {
        assert!(matches!(
            check_winfsp(),
            WinFspStatus::Available
                | WinFspStatus::Unavailable {
                    reason: WinFspUnavailableReason::RuntimeUnavailable
                }
        ));
    }

    #[test]
    fn availability_status_reports_only_available_as_available() {
        assert!(WinFspStatus::Available.is_available());
        assert!(
            !WinFspStatus::Unavailable {
                reason: WinFspUnavailableReason::RuntimeUnavailable
            }
            .is_available()
        );
    }

    #[test]
    fn read_operations_are_allowed() {
        for operation in [
            WinOperation::QueryVolumeInformation,
            WinOperation::Open,
            WinOperation::QueryFileInformation,
            WinOperation::ReadDirectory,
            WinOperation::Read,
            WinOperation::ReadLink,
            WinOperation::Cleanup,
            WinOperation::Close,
        ] {
            assert_eq!(operation.decision(), OperationDecision::AllowRead);
            assert!(ReadOnlyDispatcher::<FakeFilesystem>::authorize(operation).is_ok());
        }
    }

    #[test]
    fn every_mutating_operation_is_denied() {
        for operation in [
            WinOperation::Create,
            WinOperation::Write,
            WinOperation::SetFileSize,
            WinOperation::CanDelete,
            WinOperation::Delete,
            WinOperation::Rename,
            WinOperation::SetBasicInformation,
            WinOperation::SetSecurity,
            WinOperation::Flush,
        ] {
            let error = ReadOnlyDispatcher::<FakeFilesystem>::authorize(operation)
                .expect_err("mutating operation must be denied");
            assert_eq!(error.category(), ErrorCategory::PermissionDenied);
        }
    }

    #[test]
    fn dispatcher_forwards_only_read_operations() {
        let dispatcher = ReadOnlyDispatcher::new(FakeFilesystem);
        let path = FsPath::root();
        assert_eq!(dispatcher.info().expect("info").filesystem_type, "fake");
        assert_eq!(
            dispatcher.lookup(&path).expect("lookup").kind,
            FileKind::Directory
        );
        assert_eq!(dispatcher.read_dir(&path).expect("directory").len(), 1);
        let mut bytes = [0; 4];
        assert_eq!(
            dispatcher.read_file_at(&path, 0, &mut bytes).expect("read"),
            4
        );
        assert_eq!(&bytes, b"test");
        assert_eq!(
            dispatcher.read_link(&path).expect("link").as_str(),
            "/target"
        );
    }

    struct FakeFilesystem;

    impl ReadOnlyFilesystem for FakeFilesystem {
        fn info(&self) -> Result<FilesystemInfo> {
            Ok(FilesystemInfo {
                filesystem_type: "fake".to_owned(),
                label: None,
                uuid: None,
                block_size: None,
                total_size: None,
                free_size: None,
            })
        }

        fn lookup(&self, _path: &FsPath) -> Result<NodeMetadata> {
            Ok(NodeMetadata {
                kind: FileKind::Directory,
                size: 0,
                permissions: 0o755,
                uid: 0,
                gid: 0,
            })
        }

        fn read_dir(&self, _path: &FsPath) -> Result<Vec<DirectoryEntry>> {
            Ok(vec![DirectoryEntry {
                name: "entry".to_owned(),
                metadata: NodeMetadata {
                    kind: FileKind::Regular,
                    size: 4,
                    permissions: 0o644,
                    uid: 0,
                    gid: 0,
                },
            }])
        }

        fn read_file_at(
            &self,
            _path: &FsPath,
            _offset: u64,
            destination: &mut [u8],
        ) -> Result<usize> {
            destination.copy_from_slice(b"test");
            Ok(destination.len())
        }

        fn read_link(&self, _path: &FsPath) -> Result<FsPath> {
            FsPath::parse("/target")
        }
    }
}

/// Lifecycle state for a mounted read-only volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountStatus {
    Unmounted,
    Mounting,
    Mounted,
    Unmounting,
    Failed,
}

/// Native host operations required by the platform adapter.
///
/// The implementation is responsible for configuring WinFsp as read-only and
/// for mapping all source mutations to access denied before returning success.
pub trait MountHost {
    fn mount(&mut self) -> Result<()>;
    fn unmount(&mut self) -> Result<()>;
}

/// Owns mount lifecycle transitions without coupling the core to WinFsp types.
pub struct MountManager<H: MountHost> {
    host: H,
    status: MountStatus,
}

impl<H> MountManager<H>
where
    H: MountHost,
{
    pub fn new(host: H) -> Self {
        Self {
            host,
            status: MountStatus::Unmounted,
        }
    }

    pub fn status(&self) -> MountStatus {
        self.status
    }

    pub fn mount(&mut self) -> Result<()> {
        if self.status != MountStatus::Unmounted {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::PermissionDenied,
                "mount request is invalid in the current lifecycle state",
            ));
        }
        self.status = MountStatus::Mounting;
        match self.host.mount() {
            Ok(()) => {
                self.status = MountStatus::Mounted;
                Ok(())
            }
            Err(error) => {
                self.status = MountStatus::Failed;
                Err(error)
            }
        }
    }

    pub fn unmount(&mut self) -> Result<()> {
        if self.status != MountStatus::Mounted {
            return Err(linuxfs_core::Error::new(
                linuxfs_core::ErrorCategory::PermissionDenied,
                "unmount request is invalid in the current lifecycle state",
            ));
        }
        self.status = MountStatus::Unmounting;
        match self.host.unmount() {
            Ok(()) => {
                self.status = MountStatus::Unmounted;
                Ok(())
            }
            Err(error) => {
                // Keep ownership visible so callers can retry a teardown that
                // may have failed while the host still owns its mount point.
                self.status = MountStatus::Mounted;
                Err(error)
            }
        }
    }

    /// Attempts to clean up a mount owned by this manager during orderly shutdown.
    ///
    /// An already-unmounted manager is safely idempotent. Other non-mounted
    /// states are left untouched because the manager cannot safely infer
    /// whether the host owns a live mount.
    pub fn shutdown(&mut self) -> Result<()> {
        match self.status {
            MountStatus::Mounted => self.unmount(),
            MountStatus::Unmounted => Ok(()),
            MountStatus::Mounting | MountStatus::Unmounting | MountStatus::Failed => {
                Err(linuxfs_core::Error::new(
                    linuxfs_core::ErrorCategory::PermissionDenied,
                    "shutdown cleanup is invalid in the current lifecycle state",
                ))
            }
        }
    }
}

impl<H> Drop for MountManager<H>
where
    H: MountHost,
{
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use linuxfs_core::ErrorCategory;
    use std::cell::Cell;
    use std::rc::Rc;

    struct FakeHost {
        fail_mount: bool,
        fail_unmount: bool,
        unmount_calls: usize,
    }

    impl MountHost for FakeHost {
        fn mount(&mut self) -> Result<()> {
            if self.fail_mount {
                Err(linuxfs_core::Error::new(
                    ErrorCategory::WinFspFailure,
                    "fake mount failure",
                ))
            } else {
                Ok(())
            }
        }

        fn unmount(&mut self) -> Result<()> {
            self.unmount_calls += 1;
            if self.fail_unmount {
                Err(linuxfs_core::Error::new(
                    ErrorCategory::WinFspFailure,
                    "fake unmount failure",
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn lifecycle_requires_mount_before_unmount() {
        let mut manager = MountManager::new(FakeHost {
            fail_mount: false,
            fail_unmount: false,
            unmount_calls: 0,
        });
        assert_eq!(manager.status(), MountStatus::Unmounted);
        assert!(manager.unmount().is_err());
        manager.mount().expect("mount");
        assert_eq!(manager.status(), MountStatus::Mounted);
        manager.unmount().expect("unmount");
        assert_eq!(manager.status(), MountStatus::Unmounted);
    }

    #[test]
    fn failed_host_operation_enters_failed_state() {
        let mut manager = MountManager::new(FakeHost {
            fail_mount: true,
            fail_unmount: false,
            unmount_calls: 0,
        });
        assert_eq!(
            manager.mount().expect_err("failure").category(),
            ErrorCategory::WinFspFailure
        );
        assert_eq!(manager.status(), MountStatus::Failed);
    }

    #[test]
    fn shutdown_unmounts_owned_mount_and_is_idempotent() {
        let mut manager = MountManager::new(FakeHost {
            fail_mount: false,
            fail_unmount: false,
            unmount_calls: 0,
        });
        manager.mount().expect("mount");

        manager.shutdown().expect("shutdown");
        assert_eq!(manager.status(), MountStatus::Unmounted);
        assert_eq!(manager.host.unmount_calls, 1);

        manager.shutdown().expect("second shutdown");
        assert_eq!(manager.host.unmount_calls, 1);
    }

    #[test]
    fn shutdown_reports_cleanup_failure_and_preserves_mounted_state() {
        let mut manager = MountManager::new(FakeHost {
            fail_mount: false,
            fail_unmount: true,
            unmount_calls: 0,
        });
        manager.mount().expect("mount");

        let error = manager
            .shutdown()
            .expect_err("shutdown must report failure");
        assert_eq!(error.category(), ErrorCategory::WinFspFailure);
        assert_eq!(manager.status(), MountStatus::Mounted);
        assert_eq!(manager.host.unmount_calls, 1);
    }

    #[test]
    fn failed_unmount_retains_mounted_state_for_retry() {
        let mut manager = MountManager::new(FakeHost {
            fail_mount: false,
            fail_unmount: true,
            unmount_calls: 0,
        });
        manager.mount().expect("mount");

        assert!(manager.unmount().is_err());
        assert_eq!(manager.status(), MountStatus::Mounted);
    }

    #[test]
    fn drop_attempts_cleanup_for_a_mounted_manager() {
        let unmount_calls = Rc::new(Cell::new(0));
        let observed_calls = Rc::clone(&unmount_calls);
        {
            let mut manager = MountManager::new(DropHost { unmount_calls });
            manager.mount().expect("mount");
        }
        assert_eq!(observed_calls.get(), 1);
    }

    struct DropHost {
        unmount_calls: Rc<Cell<usize>>,
    }

    impl MountHost for DropHost {
        fn mount(&mut self) -> Result<()> {
            Ok(())
        }

        fn unmount(&mut self) -> Result<()> {
            self.unmount_calls.set(self.unmount_calls.get() + 1);
            Ok(())
        }
    }
}

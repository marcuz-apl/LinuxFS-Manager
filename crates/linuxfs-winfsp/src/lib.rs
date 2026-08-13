#[cfg(windows)]
pub mod native;

use linuxfs_core::{
    DirectoryEntry, FilesystemInfo, FsPath, NodeMetadata, ReadOnlyFilesystem, Result,
};
use std::path::PathBuf;

/// State observed for the WinFsp launcher service without changing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinFspLauncherStatus {
    NotInstalled,
    Stopped,
    Running,
    QueryFailed,
    UnsupportedPlatform,
}

/// The first WinFsp prerequisite that prevents the application from starting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinFspRequirement {
    Ready,
    UnsupportedPlatform,
    InstallationNotRegistered,
    RuntimeDllMissing,
    LauncherNotInstalled,
    LauncherNotRunning,
    LauncherStatusUnavailable,
    RuntimeInitializationFailed,
}

/// A live, diagnostic-only assessment of the installed WinFsp framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinFspAssessment {
    installation_registered: bool,
    runtime_dll_present: bool,
    launcher_status: WinFspLauncherStatus,
    runtime_initialized: bool,
}

impl WinFspAssessment {
    pub fn from_checks(
        installation_registered: bool,
        runtime_dll_present: bool,
        launcher_status: WinFspLauncherStatus,
        runtime_initialized: bool,
    ) -> Self {
        Self {
            installation_registered,
            runtime_dll_present,
            launcher_status,
            runtime_initialized,
        }
    }

    pub fn requirement(&self) -> WinFspRequirement {
        if self.launcher_status == WinFspLauncherStatus::UnsupportedPlatform {
            return WinFspRequirement::UnsupportedPlatform;
        }
        if !self.installation_registered {
            return WinFspRequirement::InstallationNotRegistered;
        }
        if !self.runtime_dll_present {
            return WinFspRequirement::RuntimeDllMissing;
        }
        match self.launcher_status {
            WinFspLauncherStatus::NotInstalled => WinFspRequirement::LauncherNotInstalled,
            WinFspLauncherStatus::Stopped => WinFspRequirement::LauncherNotRunning,
            WinFspLauncherStatus::QueryFailed => WinFspRequirement::LauncherStatusUnavailable,
            WinFspLauncherStatus::Running => {
                if self.runtime_initialized {
                    WinFspRequirement::Ready
                } else {
                    WinFspRequirement::RuntimeInitializationFailed
                }
            }
            WinFspLauncherStatus::UnsupportedPlatform => WinFspRequirement::UnsupportedPlatform,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.requirement() == WinFspRequirement::Ready
    }

    pub fn installation_registered(&self) -> bool {
        self.installation_registered
    }

    pub fn runtime_dll_present(&self) -> bool {
        self.runtime_dll_present
    }

    pub fn launcher_status(&self) -> WinFspLauncherStatus {
        self.launcher_status
    }

    pub fn runtime_initialized(&self) -> bool {
        self.runtime_initialized
    }
}

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
    if assess_winfsp().is_ready() {
        WinFspStatus::Available
    } else {
        WinFspStatus::Unavailable {
            reason: platform::unavailable_reason(),
        }
    }
}

/// Performs a live, read-only WinFsp framework assessment.
pub fn assess_winfsp() -> WinFspAssessment {
    platform::assess()
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
    use super::{WinFspAssessment, WinFspLauncherStatus, WinFspUnavailableReason};
    use std::path::PathBuf;

    pub fn prepare_runtime() -> linuxfs_core::Result<()> {
        super::load_runtime_dll()
    }

    pub fn assess() -> WinFspAssessment {
        let installation_registered = super::registry_installation_dir().is_some();
        if !installation_registered {
            return WinFspAssessment::from_checks(
                false,
                false,
                WinFspLauncherStatus::NotInstalled,
                false,
            );
        }

        let runtime_dll_present = super::winfsp_runtime_path().is_some_and(|path| path.is_file());
        if !runtime_dll_present {
            return WinFspAssessment::from_checks(
                true,
                false,
                WinFspLauncherStatus::NotInstalled,
                false,
            );
        }

        let launcher_status = super::launcher_service_status();
        if launcher_status != WinFspLauncherStatus::Running {
            return WinFspAssessment::from_checks(true, true, launcher_status, false);
        }

        let runtime_initialized = super::load_runtime_dll()
            .and_then(|()| {
                winfsp::winfsp_init().map(|_init| ()).map_err(|error| {
                    linuxfs_core::Error::with_source(
                        linuxfs_core::ErrorCategory::WinFspUnavailable,
                        "WinFsp runtime initialization failed",
                        error,
                    )
                })
            })
            .is_ok();
        WinFspAssessment::from_checks(true, true, launcher_status, runtime_initialized)
    }

    pub fn unavailable_reason() -> WinFspUnavailableReason {
        WinFspUnavailableReason::RuntimeUnavailable
    }

    pub fn installation_dir() -> Option<PathBuf> {
        super::registry_installation_dir()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::{WinFspAssessment, WinFspLauncherStatus, WinFspUnavailableReason};
    use std::path::PathBuf;

    pub fn assess() -> WinFspAssessment {
        WinFspAssessment::from_checks(
            false,
            false,
            WinFspLauncherStatus::UnsupportedPlatform,
            false,
        )
    }

    pub fn unavailable_reason() -> WinFspUnavailableReason {
        WinFspUnavailableReason::UnsupportedPlatform
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

#[cfg(windows)]
#[allow(unsafe_code)]
fn launcher_service_status() -> WinFspLauncherStatus {
    use windows_sys::Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx, SC_MANAGER_CONNECT,
        SC_STATUS_PROCESS_INFO, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS_PROCESS,
    };

    // SAFETY: null pointers select the local machine and active service database. The function
    // performs a read-only connection to the Service Control Manager.
    let manager = unsafe { OpenSCManagerW(std::ptr::null(), std::ptr::null(), SC_MANAGER_CONNECT) };
    if manager.is_null() {
        return WinFspLauncherStatus::QueryFailed;
    }
    let service_name: Vec<u16> = "WinFsp.Launcher"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `manager` is a valid SCM handle and `service_name` is a valid, NUL-terminated
    // UTF-16 buffer. The requested access only permits querying service status.
    let service = unsafe { OpenServiceW(manager, service_name.as_ptr(), SERVICE_QUERY_STATUS) };
    if service.is_null() {
        // SAFETY: `manager` was obtained from OpenSCManagerW and is released exactly once here.
        unsafe { CloseServiceHandle(manager) };
        return WinFspLauncherStatus::NotInstalled;
    }

    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut bytes_needed = 0_u32;
    // SAFETY: `service` is a valid service handle. `status` is an initialized writable buffer of
    // the advertised size and `bytes_needed` points to writable memory. The call only queries.
    let queried = unsafe {
        QueryServiceStatusEx(
            service,
            SC_STATUS_PROCESS_INFO,
            (&mut status as *mut SERVICE_STATUS_PROCESS).cast(),
            std::mem::size_of::<SERVICE_STATUS_PROCESS>() as u32,
            &mut bytes_needed,
        )
    };
    // SAFETY: both handles were obtained above and each is released exactly once after the query.
    unsafe {
        CloseServiceHandle(service);
        CloseServiceHandle(manager);
    }
    if queried == 0 {
        return WinFspLauncherStatus::QueryFailed;
    }
    if status.dwCurrentState == SERVICE_RUNNING {
        WinFspLauncherStatus::Running
    } else {
        WinFspLauncherStatus::Stopped
    }
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
    fn assessment_requires_every_framework_component() {
        assert_eq!(
            WinFspAssessment::from_checks(true, true, WinFspLauncherStatus::Running, true,)
                .requirement(),
            WinFspRequirement::Ready
        );
        assert_eq!(
            WinFspAssessment::from_checks(true, true, WinFspLauncherStatus::Stopped, true,)
                .requirement(),
            WinFspRequirement::LauncherNotRunning
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

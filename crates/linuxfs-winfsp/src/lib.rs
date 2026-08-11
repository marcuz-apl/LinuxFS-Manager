use linuxfs_core::{
    DirectoryEntry, FilesystemInfo, FsPath, NodeMetadata, ReadOnlyFilesystem, Result,
};

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

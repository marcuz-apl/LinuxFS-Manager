use linuxfs_core::FileKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Image,
    PhysicalDisk,
    Partition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatus {
    Detected,
    Compatible,
    Incompatible,
    Mounted,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceViewModel {
    pub id: SourceId,
    pub kind: SourceKind,
    pub display_name: String,
    pub source_description: String,
    pub filesystem_type: Option<String>,
    pub label: Option<String>,
    pub uuid: Option<String>,
    pub size_bytes: Option<u64>,
    pub status: SourceStatus,
    pub mount_point: Option<String>,
    pub read_only: bool,
}

impl SourceViewModel {
    pub fn can_mount(&self) -> bool {
        self.read_only
            && matches!(self.status, SourceStatus::Compatible)
            && self.mount_point.is_none()
    }

    pub fn can_unmount(&self) -> bool {
        matches!(self.status, SourceStatus::Mounted) && self.mount_point.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    Refresh,
    OpenImage,
    Mount(SourceId),
    Unmount(SourceId),
    OpenInExplorer(SourceId),
    ShowDetails(SourceId),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppModel {
    sources: Vec<SourceViewModel>,
    busy: bool,
    message: Option<String>,
}

impl AppModel {
    pub fn sources(&self) -> &[SourceViewModel] {
        &self.sources
    }

    pub fn busy(&self) -> bool {
        self.busy
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn replace_sources(&mut self, sources: Vec<SourceViewModel>) {
        self.sources = sources;
    }

    pub fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
    }

    pub fn set_message(&mut self, message: Option<String>) {
        self.message = message;
    }

    pub fn source_mut(&mut self, id: SourceId) -> Option<&mut SourceViewModel> {
        self.sources.iter_mut().find(|source| source.id == id)
    }
}

/// Convert filesystem metadata into UI-safe display information without
/// exposing parser-specific types to the application layer.
pub fn display_kind(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Regular => "File",
        FileKind::Directory => "Folder",
        FileKind::Symlink => "Symbolic link",
        FileKind::Other => "Special file",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compatible_source() -> SourceViewModel {
        SourceViewModel {
            id: SourceId(1),
            kind: SourceKind::Image,
            display_name: "fixture.img".to_owned(),
            source_description: "Raw image".to_owned(),
            filesystem_type: Some("ext4".to_owned()),
            label: Some("fixture".to_owned()),
            uuid: None,
            size_bytes: Some(32 * 1024 * 1024),
            status: SourceStatus::Compatible,
            mount_point: None,
            read_only: true,
        }
    }

    #[test]
    fn source_commands_reflect_read_only_mount_state() {
        let mut source = compatible_source();
        assert!(source.can_mount());
        assert!(!source.can_unmount());

        source.status = SourceStatus::Mounted;
        source.mount_point = Some("L:".to_owned());
        assert!(!source.can_mount());
        assert!(source.can_unmount());
    }

    #[test]
    fn a_source_can_never_be_mountable_when_read_only_is_false() {
        let mut source = compatible_source();
        source.read_only = false;
        assert!(!source.can_mount());
    }

    #[test]
    fn model_replaces_sources_and_tracks_busy_message() {
        let mut model = AppModel::default();
        model.replace_sources(vec![compatible_source()]);
        model.set_busy(true);
        model.set_message(Some("Scanning".to_owned()));
        assert_eq!(model.sources().len(), 1);
        assert!(model.busy());
        assert_eq!(model.message(), Some("Scanning"));
    }
}

pub trait SourceProvider {
    fn refresh(&mut self) -> linuxfs_core::Result<Vec<SourceViewModel>>;
    fn open_image(&mut self, path: &str) -> linuxfs_core::Result<SourceViewModel>;
}

pub trait MountService {
    fn mount(&mut self, source: &SourceViewModel) -> linuxfs_core::Result<String>;
    fn unmount(&mut self, source: &SourceViewModel) -> linuxfs_core::Result<()>;
    fn open_in_explorer(&mut self, mount_point: &str) -> linuxfs_core::Result<()>;
}

pub struct AppController<P, M> {
    model: AppModel,
    provider: P,
    mount_service: M,
}

impl<P, M> AppController<P, M>
where
    P: SourceProvider,
    M: MountService,
{
    pub fn new(provider: P, mount_service: M) -> Self {
        Self {
            model: AppModel::default(),
            provider,
            mount_service,
        }
    }

    pub fn model(&self) -> &AppModel {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut AppModel {
        &mut self.model
    }

    pub fn refresh(&mut self) -> linuxfs_core::Result<()> {
        self.model.set_busy(true);
        self.model.set_message(Some("Scanning sources…".to_owned()));
        let result = self.provider.refresh();
        match result {
            Ok(sources) => {
                self.model.replace_sources(sources);
                self.finish(Ok(()))
            }
            Err(error) => self.finish(Err(error)),
        }
    }

    pub fn open_image(&mut self, path: &str) -> linuxfs_core::Result<()> {
        self.model.set_busy(true);
        self.model.set_message(Some("Opening image…".to_owned()));
        let result = self.provider.open_image(path);
        match result {
            Ok(source) => {
                self.model.sources.push(source);
                self.finish(Ok(()))
            }
            Err(error) => self.finish(Err(error)),
        }
    }

    pub fn mount(&mut self, id: SourceId) -> linuxfs_core::Result<()> {
        let source = self.source(id)?.clone();
        if !source.can_mount() {
            return Err(command_error(
                "source is not mountable in its current state",
            ));
        }
        self.model.set_busy(true);
        self.model
            .set_message(Some("Mounting read-only…".to_owned()));
        let result = self.mount_service.mount(&source);
        match result {
            Ok(mount_point) => {
                if let Some(source) = self.model.source_mut(id) {
                    source.status = SourceStatus::Mounted;
                    source.mount_point = Some(mount_point);
                    source.read_only = true;
                }
                self.finish(Ok(()))
            }
            Err(error) => self.finish(Err(error)),
        }
    }

    pub fn unmount(&mut self, id: SourceId) -> linuxfs_core::Result<()> {
        let source = self.source(id)?.clone();
        if !source.can_unmount() {
            return Err(command_error("source is not mounted"));
        }
        self.model.set_busy(true);
        self.model.set_message(Some("Unmounting…".to_owned()));
        let result = self.mount_service.unmount(&source);
        match result {
            Ok(()) => {
                if let Some(source) = self.model.source_mut(id) {
                    source.status = SourceStatus::Compatible;
                    source.mount_point = None;
                    source.read_only = true;
                }
                self.finish(Ok(()))
            }
            Err(error) => self.finish(Err(error)),
        }
    }

    pub fn open_in_explorer(&mut self, id: SourceId) -> linuxfs_core::Result<()> {
        let mount_point = self
            .source(id)?
            .mount_point
            .clone()
            .ok_or_else(|| command_error("source has no mount point"))?;
        self.mount_service.open_in_explorer(&mount_point)
    }

    fn source(&self, id: SourceId) -> linuxfs_core::Result<&SourceViewModel> {
        self.model
            .sources
            .iter()
            .find(|source| source.id == id)
            .ok_or_else(|| command_error("source was not found"))
    }

    fn finish<T>(&mut self, result: linuxfs_core::Result<T>) -> linuxfs_core::Result<T> {
        self.model.set_busy(false);
        if result.is_ok() {
            self.model.set_message(None);
        } else {
            self.model.set_message(Some("Operation failed".to_owned()));
        }
        result
    }
}

fn command_error(message: &'static str) -> linuxfs_core::Error {
    linuxfs_core::Error::new(linuxfs_core::ErrorCategory::Internal, message)
}
#[cfg(test)]
mod controller_tests {
    use super::*;

    struct FakeProvider;

    impl SourceProvider for FakeProvider {
        fn refresh(&mut self) -> linuxfs_core::Result<Vec<SourceViewModel>> {
            Ok(vec![test_source()])
        }

        fn open_image(&mut self, path: &str) -> linuxfs_core::Result<SourceViewModel> {
            let mut source = test_source();
            source.display_name = path.to_owned();
            Ok(source)
        }
    }

    struct FakeMount;

    impl MountService for FakeMount {
        fn mount(&mut self, _source: &SourceViewModel) -> linuxfs_core::Result<String> {
            Ok("L:".to_owned())
        }

        fn unmount(&mut self, _source: &SourceViewModel) -> linuxfs_core::Result<()> {
            Ok(())
        }

        fn open_in_explorer(&mut self, _mount_point: &str) -> linuxfs_core::Result<()> {
            Ok(())
        }
    }

    fn test_source() -> SourceViewModel {
        SourceViewModel {
            id: SourceId(7),
            kind: SourceKind::Image,
            display_name: "fixture.img".to_owned(),
            source_description: "Raw image".to_owned(),
            filesystem_type: Some("ext4".to_owned()),
            label: None,
            uuid: None,
            size_bytes: Some(1024),
            status: SourceStatus::Compatible,
            mount_point: None,
            read_only: true,
        }
    }

    #[test]
    fn controller_routes_refresh_mount_and_unmount() {
        let mut controller = AppController::new(FakeProvider, FakeMount);
        controller.refresh().expect("refresh");
        controller.mount(SourceId(7)).expect("mount");
        assert!(controller.model().sources()[0].can_unmount());
        controller.unmount(SourceId(7)).expect("unmount");
        assert!(controller.model().sources()[0].can_mount());
    }

    #[test]
    fn controller_rejects_mounting_incompatible_source() {
        let mut controller = AppController::new(FakeProvider, FakeMount);
        controller
            .model_mut()
            .replace_sources(vec![SourceViewModel {
                status: SourceStatus::Incompatible,
                ..test_source()
            }]);
        let error = controller
            .mount(SourceId(7))
            .expect_err("incompatible source");
        assert_eq!(error.category(), linuxfs_core::ErrorCategory::Internal);
    }
}

#[cfg(windows)]
pub struct ImageSourceProvider {
    next_id: u64,
}

#[cfg(windows)]
impl Default for ImageSourceProvider {
    fn default() -> Self {
        Self { next_id: 1 }
    }
}

#[cfg(windows)]
impl ImageSourceProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(windows)]
impl ImageSourceProvider {
    fn source_from_info(
        &mut self,
        path: &str,
        size_bytes: u64,
        description: String,
        info: linuxfs_core::FilesystemInfo,
    ) -> SourceViewModel {
        use std::path::Path;
        let id = SourceId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let display_name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_owned();
        SourceViewModel {
            id,
            kind: if description == path {
                SourceKind::Image
            } else {
                SourceKind::Partition
            },
            display_name,
            source_description: description,
            filesystem_type: Some(info.filesystem_type),
            label: info.label,
            uuid: info
                .uuid
                .map(|uuid| uuid.iter().map(|byte| format!("{byte:02x}")).collect()),
            size_bytes: Some(size_bytes),
            status: SourceStatus::Compatible,
            mount_point: None,
            read_only: true,
        }
    }
}

#[cfg(windows)]
impl SourceProvider for ImageSourceProvider {
    fn refresh(&mut self) -> linuxfs_core::Result<Vec<SourceViewModel>> {
        Ok(Vec::new())
    }

    fn open_image(&mut self, path: &str) -> linuxfs_core::Result<SourceViewModel> {
        use linuxfs_core::{
            BlockGeometry, BlockReader, PartitionReader, SourceLayout, discover_layout,
        };
        use linuxfs_ext::ExtReadOnlyBackend;
        use linuxfs_storage::RawImageReader;
        use std::sync::Arc;

        let reader: Arc<dyn BlockReader> = Arc::new(RawImageReader::open(path)?);
        let source_size = reader.len()?;
        let layout = discover_layout(reader.as_ref(), BlockGeometry::raw_image_512())?;
        match layout {
            SourceLayout::DirectImage => {
                let backend = ExtReadOnlyBackend::open(Arc::clone(&reader))?;
                let info = backend.info()?;
                Ok(self.source_from_info(path, source_size, path.to_owned(), info))
            }
            SourceLayout::Mbr { partitions } | SourceLayout::Gpt { partitions } => {
                let mut last_error = None;
                for partition in partitions {
                    let view = match PartitionReader::new(
                        Arc::clone(&reader),
                        partition.byte_offset,
                        partition.byte_length,
                    ) {
                        Ok(view) => Arc::new(view) as Arc<dyn BlockReader>,
                        Err(error) => {
                            last_error = Some(error);
                            continue;
                        }
                    };
                    match ExtReadOnlyBackend::open(Arc::clone(&view)) {
                        Ok(backend) => {
                            let info = backend.info()?;
                            return Ok(self.source_from_info(
                                path,
                                partition.byte_length,
                                format!("{path} (partition {})", partition.number),
                                info,
                            ));
                        }
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    linuxfs_core::Error::new(
                        linuxfs_core::ErrorCategory::UnsupportedFilesystem,
                        "no supported Ext filesystem found in image partitions",
                    )
                }))
            }
        }
    }
}

#[cfg(windows)]
pub struct WindowsSourceProvider {
    image_provider: ImageSourceProvider,
}

#[cfg(windows)]
impl Default for WindowsSourceProvider {
    fn default() -> Self {
        Self {
            image_provider: ImageSourceProvider::new(),
        }
    }
}

#[cfg(windows)]
impl WindowsSourceProvider {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(windows)]
impl SourceProvider for WindowsSourceProvider {
    fn refresh(&mut self) -> linuxfs_core::Result<Vec<SourceViewModel>> {
        Ok(linuxfs_windows::discover_physical_disks(32)
            .into_iter()
            .map(|disk| SourceViewModel {
                id: SourceId((1_u64 << 63) | u64::from(disk.index)),
                kind: SourceKind::PhysicalDisk,
                display_name: format!("PhysicalDrive{}", disk.index),
                source_description: disk.source_path.to_string_lossy().into_owned(),
                filesystem_type: None,
                label: None,
                uuid: None,
                size_bytes: Some(disk.size_bytes),
                status: SourceStatus::Detected,
                mount_point: None,
                read_only: true,
            })
            .collect())
    }

    fn open_image(&mut self, path: &str) -> linuxfs_core::Result<SourceViewModel> {
        self.image_provider.open_image(path)
    }
}

#[cfg(windows)]
#[cfg(test)]
mod provided_image_tests {
    use super::*;

    #[test]
    fn configured_image_probes_read_only() {
        let Some(path) = std::env::var_os("LINUXFS_TEST_IMAGE") else {
            return;
        };
        let mut provider = ImageSourceProvider::new();
        let source = provider
            .open_image(&path.to_string_lossy())
            .expect("configured image should be probeable");
        assert_eq!(source.status, SourceStatus::Compatible);
        assert!(source.read_only);
    }
}

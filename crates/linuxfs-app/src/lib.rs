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

use std::{fs, path::PathBuf, sync::Arc};

use linuxfs_core::{BlockReader, FsPath, ReadOnlyFilesystem};
use linuxfs_ext::ExtReadOnlyBackend;
use linuxfs_storage::RawImageReader;

#[test]
fn generated_ext_fixtures_are_read_only_and_probeable() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(r"..\\..\\tests\\fixtures-ext\\generated");
    let required = std::env::var_os("LINUXFS_REQUIRE_FIXTURES").is_some();
    let mut checked = 0;
    for name in ["ext2.img", "ext3.img", "ext4.img"] {
        let path = root.join(name);
        if !path.exists() {
            assert!(!required, "missing required fixture {}", path.display());
            continue;
        }
        let before = fs::read(&path).expect("fixture reads before probe");
        let reader = RawImageReader::open(&path).expect("fixture opens read-only");
        let reader: Arc<dyn BlockReader> = Arc::new(reader);
        let backend = ExtReadOnlyBackend::open(Arc::clone(&reader))
            .unwrap_or_else(|error| panic!("{name} rejected: {error}"));
        let info = backend.info().expect("filesystem info reads");
        assert_eq!(info.filesystem_type, "ext2/ext3/ext4");
        let root_path = FsPath::root();
        let metadata = backend.lookup(&root_path).expect("root metadata reads");
        assert!(metadata.kind == linuxfs_core::FileKind::Directory);
        let entries = backend.read_dir(&root_path).expect("root directory reads");
        assert!(
            entries.iter().any(|entry| entry.name == "lost+found"),
            "fresh fixture should expose lost+found"
        );
        let file_path = FsPath::parse("/hello.txt").expect("fixture file path");
        let file_metadata = backend.lookup(&file_path).expect("file metadata reads");
        assert_eq!(file_metadata.kind, linuxfs_core::FileKind::Regular);
        let mut content = [0; 16];
        let content_len = backend
            .read_file_at(&file_path, 0, &mut content)
            .expect("file reads");
        assert_eq!(&content[..content_len], b"linuxfs-fixture-");
        let link_path = FsPath::parse("/hello-link").expect("fixture link path");
        assert_eq!(
            backend
                .read_link(&link_path)
                .expect("symlink reads")
                .as_str(),
            "/hello.txt"
        );
        drop(backend);
        let after = fs::read(&path).expect("fixture reads after probe");
        assert_eq!(after, before, "{name} changed during read-only probe");
        checked += 1;
    }
    if required {
        assert!(checked > 0, "no generated Ext fixtures found");
    }
}

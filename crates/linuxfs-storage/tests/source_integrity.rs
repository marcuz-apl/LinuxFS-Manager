use linuxfs_core::{BlockGeometry, ErrorCategory, SourceLayout, discover_layout};
use linuxfs_storage::RawImageReader;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, Read},
    panic::catch_unwind,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_IMAGE_ID: AtomicU64 = AtomicU64::new(0);

struct TempImage {
    path: PathBuf,
}

impl TempImage {
    fn new(bytes: &[u8]) -> Self {
        let id = NEXT_IMAGE_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linuxfs-manager-integrity-{}-{}",
            std::process::id(),
            id
        ));
        fs::write(&path, bytes).expect("fixture writes");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempImage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn hash_file(path: &Path) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut hasher = DefaultHasher::new();
    let mut buffer = [0; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        buffer[..read].hash(&mut hasher);
    }
    Ok(hasher.finish())
}

fn mbr_image() -> Vec<u8> {
    let mut bytes = vec![0; 16 * 512];
    bytes[510..512].copy_from_slice(&[0x55, 0xAA]);
    bytes[446 + 4] = 0x83;
    bytes[446 + 8..446 + 12].copy_from_slice(&2u32.to_le_bytes());
    bytes[446 + 12..446 + 16].copy_from_slice(&4u32.to_le_bytes());
    bytes
}

fn malformed_gpt_image() -> Vec<u8> {
    let mut bytes = vec![0; 16 * 512];
    bytes[512..520].copy_from_slice(b"EFI PART");
    bytes
}

#[test]
fn inspection_preserves_source_image_exactly() {
    let image = TempImage::new(&mbr_image());
    let bytes_before = fs::read(image.path()).expect("fixture reads before inspection");
    let hash_before = hash_file(image.path()).expect("fixture hashes before inspection");

    let reader = RawImageReader::open(image.path()).expect("image opens read-only");
    let layout = discover_layout(&reader, BlockGeometry::raw_image_512())
        .expect("layout discovery succeeds");
    assert!(matches!(layout, SourceLayout::Mbr { .. }));
    drop(reader);

    let hash_after = hash_file(image.path()).expect("fixture hashes after inspection");
    let bytes_after = fs::read(image.path()).expect("fixture reads after inspection");
    assert_eq!(hash_after, hash_before);
    assert_eq!(bytes_after, bytes_before);
}

#[test]
fn malformed_gpt_returns_error_without_panic_or_mutation() {
    let image = TempImage::new(&malformed_gpt_image());
    let bytes_before = fs::read(image.path()).expect("fixture reads before inspection");
    let hash_before = hash_file(image.path()).expect("fixture hashes before inspection");

    let outcome = catch_unwind(|| {
        let reader = RawImageReader::open(image.path()).expect("image opens read-only");
        discover_layout(&reader, BlockGeometry::raw_image_512())
    });
    let result = outcome.expect("malformed GPT must not panic");
    assert_eq!(
        result.expect_err("malformed GPT is rejected").category(),
        ErrorCategory::PartitionTable
    );

    assert_eq!(
        hash_file(image.path()).expect("fixture hashes after inspection"),
        hash_before
    );
    assert_eq!(
        fs::read(image.path()).expect("fixture reads after inspection"),
        bytes_before
    );
}

#[test]
fn malformed_mbr_returns_error_without_panic_or_mutation() {
    let mut bytes = mbr_image();
    bytes[446 + 4] = 0x83;
    bytes[446 + 12..446 + 16].copy_from_slice(&0u32.to_le_bytes());
    let image = TempImage::new(&bytes);
    let bytes_before = fs::read(image.path()).expect("fixture reads before inspection");
    let hash_before = hash_file(image.path()).expect("fixture hashes before inspection");

    let outcome = catch_unwind(|| {
        let reader = RawImageReader::open(image.path()).expect("image opens read-only");
        discover_layout(&reader, BlockGeometry::raw_image_512())
    });
    let result = outcome.expect("malformed MBR must not panic");
    assert_eq!(
        result.expect_err("malformed MBR is rejected").category(),
        ErrorCategory::PartitionTable
    );

    assert_eq!(
        hash_file(image.path()).expect("fixture hashes after inspection"),
        hash_before
    );
    assert_eq!(
        fs::read(image.path()).expect("fixture reads after inspection"),
        bytes_before
    );
}

#[allow(dead_code)]
fn _assert_open_mode_is_read_only(path: &Path) -> io::Result<()> {
    let _file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(path)?;
    Ok(())
}

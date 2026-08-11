use linuxfs_core::{BlockReader, Error, ErrorCategory, Result, validate_read_range};
use std::os::windows::fs::FileExt;
use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};

#[derive(Debug)]
pub struct RawImageReader {
    file: File,
    len: u64,
}

impl RawImageReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(path.as_ref())
            .map_err(|source| map_io_error("cannot open image", source))?;
        let len = file
            .metadata()
            .map_err(|source| map_io_error("cannot read image metadata", source))?
            .len();
        Ok(Self { file, len })
    }
}

impl BlockReader for RawImageReader {
    fn len(&self) -> Result<u64> {
        Ok(self.len)
    }

    fn read_exact_at(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
        validate_read_range(self.len, offset, destination.len())?;
        let mut read_total = 0usize;
        while read_total < destination.len() {
            let read_offset = offset
                .checked_add(u64::try_from(read_total).map_err(|_| {
                    Error::new(ErrorCategory::StorageAccess, "read offset does not fit u64")
                })?)
                .ok_or_else(|| Error::new(ErrorCategory::StorageAccess, "read offset overflow"))?;
            let read = match self
                .file
                .seek_read(&mut destination[read_total..], read_offset)
            {
                Ok(read) => read,
                Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
                Err(source) => return Err(map_io_error("cannot read image", source)),
            };
            if read == 0 {
                let source =
                    io::Error::new(io::ErrorKind::UnexpectedEof, "image ended during read");
                return Err(Error::with_source(
                    ErrorCategory::StorageAccess,
                    "image ended during read",
                    source,
                ));
            }
            read_total += read;
        }
        Ok(())
    }
}

fn map_io_error(context: &'static str, source: io::Error) -> Error {
    let category = if source.kind() == io::ErrorKind::PermissionDenied {
        ErrorCategory::PermissionDenied
    } else {
        ErrorCategory::StorageAccess
    };
    Error::with_source(category, context, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
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
                "linuxfs-manager-image-{}-{}",
                std::process::id(),
                id
            ));
            fs::write(&path, bytes).expect("test image writes");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    // Test-only cleanup restores the temporary file's original writable state.
    #[allow(clippy::permissions_set_readonly_false)]
    impl Drop for TempImage {
        fn drop(&mut self) {
            let mut permissions = match fs::metadata(&self.path) {
                Ok(metadata) => metadata.permissions(),
                Err(_) => return,
            };
            permissions.set_readonly(false);
            let _ = fs::set_permissions(&self.path, permissions);
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn reads_at_zero_and_the_final_valid_byte() {
        let image = TempImage::new(&[10, 20, 30, 40]);
        let reader = RawImageReader::open(image.path()).expect("test image opens");
        let mut first = [0; 2];
        reader
            .read_exact_at(0, &mut first)
            .expect("first bytes read");
        assert_eq!(first, [10, 20]);
        let mut last = [0; 1];
        reader.read_exact_at(3, &mut last).expect("last byte reads");
        assert_eq!(last, [40]);
    }

    #[test]
    fn rejects_reads_past_end_without_changing_destination() {
        let image = TempImage::new(&[1, 2, 3, 4]);
        let reader = RawImageReader::open(image.path()).expect("test image opens");
        let mut destination = [0xA5; 2];
        let error = reader
            .read_exact_at(3, &mut destination)
            .expect_err("range is rejected");
        assert_eq!(error.category(), ErrorCategory::StorageAccess);
        assert_eq!(destination, [0xA5; 2]);
    }

    #[test]
    fn accepts_empty_read_at_eof() {
        let image = TempImage::new(&[1, 2, 3, 4]);
        let reader = RawImageReader::open(image.path()).expect("test image opens");
        reader
            .read_exact_at(4, &mut [])
            .expect("empty read at EOF succeeds");
    }

    #[test]
    fn missing_path_is_a_storage_error() {
        let path =
            std::env::temp_dir().join(format!("linuxfs-manager-missing-{}", std::process::id()));
        let error = RawImageReader::open(path).expect_err("missing image is rejected");
        assert_eq!(error.category(), ErrorCategory::StorageAccess);
    }

    #[test]
    fn opens_a_readonly_file() {
        let image = TempImage::new(&[9, 8, 7]);
        let mut permissions = fs::metadata(image.path())
            .expect("test metadata reads")
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(image.path(), permissions).expect("readonly flag sets");
        let reader = RawImageReader::open(image.path()).expect("readonly image opens");
        assert_eq!(reader.len().expect("length reads"), 3);
    }
}

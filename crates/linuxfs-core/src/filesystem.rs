use std::fmt;

use crate::{Error, ErrorCategory, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsPath(String);

impl FsPath {
    pub fn root() -> Self {
        Self("/".to_owned())
    }

    pub fn parse(path: &str) -> Result<Self> {
        if !path.starts_with('/') || path.as_bytes().contains(&0) {
            return Err(Error::new(
                ErrorCategory::InvalidImage,
                "filesystem path must be absolute",
            ));
        }
        let mut normalized = Vec::new();
        for component in path.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    return Err(Error::new(
                        ErrorCategory::InvalidImage,
                        "filesystem path escapes root",
                    ));
                }
                component => normalized.push(component),
            }
        }
        let value = if normalized.is_empty() {
            "/".to_owned()
        } else {
            format!("/{}", normalized.join("/"))
        };
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FsPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Regular,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemInfo {
    pub filesystem_type: String,
    pub label: Option<String>,
    pub uuid: Option<[u8; 16]>,
    pub block_size: Option<u32>,
    pub total_size: Option<u64>,
    pub free_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeMetadata {
    pub kind: FileKind,
    pub size: u64,
    pub permissions: u16,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub metadata: NodeMetadata,
}

pub trait ReadOnlyFilesystem {
    fn info(&self) -> Result<FilesystemInfo>;
    fn lookup(&self, path: &FsPath) -> Result<NodeMetadata>;
    fn read_dir(&self, path: &FsPath) -> Result<Vec<DirectoryEntry>>;
    fn read_file_at(&self, path: &FsPath, offset: u64, destination: &mut [u8]) -> Result<usize>;
    fn read_link(&self, path: &FsPath) -> Result<FsPath>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_are_absolute_and_root_bounded() {
        assert_eq!(
            FsPath::parse("/a/./b").expect("valid path").as_str(),
            "/a/b"
        );
        assert!(FsPath::parse("relative").is_err());
        assert!(FsPath::parse("/a/../b").is_err());
    }
}

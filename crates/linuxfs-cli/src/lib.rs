use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use linuxfs_backends::ReadOnlyBackend;
use linuxfs_core::{
    BlockGeometry, BlockReader, DirectoryEntry, Error, ErrorCategory, FsPath, ReadOnlyFilesystem,
    Result, SourceLayout, discover_layout,
};
use linuxfs_storage::RawImageReader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Inspect { image: PathBuf },
    List { image: PathBuf, path: FsPath },
    Cat { image: PathBuf, path: FsPath },
}

pub fn parse_args(args: &[String]) -> std::result::Result<Command, String> {
    let command = args.first().ok_or_else(usage)?;
    match command.as_str() {
        "inspect" => {
            let image = required_argument(args, 1, "inspect requires an image path")?;
            ensure_no_extra(args, 2)?;
            Ok(Command::Inspect {
                image: PathBuf::from(image),
            })
        }
        "ls" | "list" => {
            let image = required_argument(args, 1, "ls requires an image path")?;
            let path = args.get(2).map(String::as_str).unwrap_or("/");
            ensure_no_extra(args, 3)?;
            let path = FsPath::parse(path).map_err(|error| error.to_string())?;
            Ok(Command::List {
                image: PathBuf::from(image),
                path,
            })
        }
        "cat" => {
            let image = required_argument(args, 1, "cat requires an image path")?;
            let path = required_argument(args, 2, "cat requires a filesystem path")?;
            ensure_no_extra(args, 3)?;
            let path = FsPath::parse(&path).map_err(|error| error.to_string())?;
            Ok(Command::Cat {
                image: PathBuf::from(image),
                path,
            })
        }
        "help" | "--help" | "-h" => Err(usage()),
        _ => Err(format!("unknown command `{command}`\n{}", usage())),
    }
}

pub fn run(command: Command, output: &mut impl Write) -> Result<()> {
    match command {
        Command::Inspect { image } => {
            let filesystem = open_filesystem(&image)?;
            let info = filesystem.backend.info()?;
            write_line(output, &format!("source={}\n", image.display()))?;
            write_line(output, &format!("filesystem={}\n", info.filesystem_type))?;
            if let Some(label) = info.label {
                write_line(output, &format!("label={label}\n"))?;
            }
            if let Some(uuid) = info.uuid {
                write_line(output, &format!("uuid={}\n", hex_uuid(uuid)))?;
            }
            if let Some(block_size) = info.block_size {
                write_line(output, &format!("block_size={block_size}\n"))?;
            }
            if let Some(total_size) = info.total_size {
                write_line(output, &format!("total_size={total_size}\n"))?;
            }
            if let Some(free_size) = info.free_size {
                write_line(output, &format!("free_size={free_size}\n"))?;
            }
            write_line(output, "access=read-only\n")?;
            Ok(())
        }
        Command::List { image, path } => {
            let filesystem = open_filesystem(&image)?;
            for entry in filesystem.backend.read_dir(&path)? {
                write_line(output, &format!("{}\t{}\n", entry_kind(&entry), entry.name))?;
            }
            Ok(())
        }
        Command::Cat { image, path } => {
            let filesystem = open_filesystem(&image)?;
            stream_file(&filesystem.backend, &path, output)
        }
    }
}

struct OpenFilesystem {
    backend: ReadOnlyBackend,
}

fn open_filesystem(path: &Path) -> Result<OpenFilesystem> {
    let reader: Arc<dyn BlockReader> = Arc::new(RawImageReader::open(path)?);
    let layout = discover_layout(reader.as_ref(), BlockGeometry::raw_image_512())?;
    match layout {
        SourceLayout::DirectImage => Ok(OpenFilesystem {
            backend: ReadOnlyBackend::open(reader)?,
        }),
        SourceLayout::Mbr { partitions } | SourceLayout::Gpt { partitions } => {
            let mut last_error = None;
            for partition in partitions {
                let view = match linuxfs_core::PartitionReader::new(
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
                match ReadOnlyBackend::open(view) {
                    Ok(backend) => return Ok(OpenFilesystem { backend }),
                    Err(error) => last_error = Some(error),
                }
            }
            Err(last_error.unwrap_or_else(|| {
                Error::new(
                    ErrorCategory::UnsupportedFilesystem,
                    "no supported filesystem found in image partitions",
                )
            }))
        }
    }
}

fn stream_file(
    filesystem: &impl ReadOnlyFilesystem,
    path: &FsPath,
    output: &mut impl Write,
) -> Result<()> {
    let metadata = filesystem.lookup(path)?;
    if metadata.kind != linuxfs_core::FileKind::Regular {
        return Err(Error::new(
            ErrorCategory::InvalidImage,
            "cat requires a regular file",
        ));
    }
    let mut offset = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    while offset < metadata.size {
        let remaining = metadata.size - offset;
        let requested = usize::try_from(remaining)
            .unwrap_or(buffer.len())
            .min(buffer.len());
        let count = filesystem.read_file_at(path, offset, &mut buffer[..requested])?;
        if count == 0 {
            return Err(Error::new(
                ErrorCategory::FilesystemCorrupt,
                "file ended before its recorded size",
            ));
        }
        output.write_all(&buffer[..count]).map_err(|source| {
            Error::with_source(
                ErrorCategory::StorageAccess,
                "cannot write command output",
                source,
            )
        })?;
        offset = offset
            .checked_add(u64::try_from(count).map_err(|_| {
                Error::new(ErrorCategory::Internal, "file read count does not fit u64")
            })?)
            .ok_or_else(|| Error::new(ErrorCategory::Internal, "file offset overflow"))?;
    }
    Ok(())
}

fn entry_kind(entry: &DirectoryEntry) -> &'static str {
    match entry.metadata.kind {
        linuxfs_core::FileKind::Directory => "dir",
        linuxfs_core::FileKind::Regular => "file",
        linuxfs_core::FileKind::Symlink => "link",
        linuxfs_core::FileKind::Other => "other",
    }
}

fn hex_uuid(uuid: [u8; 16]) -> String {
    uuid.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn required_argument(
    args: &[String],
    index: usize,
    message: &str,
) -> std::result::Result<String, String> {
    args.get(index).cloned().ok_or_else(|| message.to_owned())
}

fn ensure_no_extra(args: &[String], first_extra: usize) -> std::result::Result<(), String> {
    if args.len() > first_extra {
        Err(format!("unexpected argument `{}`", args[first_extra]))
    } else {
        Ok(())
    }
}

fn usage() -> String {
    "usage: linuxfs <inspect IMAGE | ls IMAGE [PATH] | cat IMAGE PATH>".to_owned()
}

fn write_line(output: &mut impl Write, line: &str) -> Result<()> {
    output.write_all(line.as_bytes()).map_err(|source| {
        Error::with_source(
            ErrorCategory::StorageAccess,
            "cannot write command output",
            source,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inspect_command() {
        assert_eq!(
            parse_args(&["inspect".to_owned(), "disk.img".to_owned()]).expect("inspect command"),
            Command::Inspect {
                image: "disk.img".into(),
            }
        );
    }

    #[test]
    fn parses_list_with_a_root_default() {
        assert_eq!(
            parse_args(&["ls".to_owned(), "disk.img".to_owned()]).expect("list command"),
            Command::List {
                image: "disk.img".into(),
                path: FsPath::root(),
            }
        );
    }

    #[test]
    fn rejects_paths_that_escape_the_filesystem() {
        let error = parse_args(&[
            "cat".to_owned(),
            "disk.img".to_owned(),
            "/../secret".to_owned(),
        ])
        .expect_err("path traversal");
        assert!(error.contains("escapes root"));
    }
}

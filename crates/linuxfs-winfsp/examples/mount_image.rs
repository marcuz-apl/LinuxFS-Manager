//! Runtime smoke test for a read-only Ext image mounted through WinFsp.

#[cfg(not(windows))]
fn main() {
    eprintln!("mount_image requires Windows and WinFsp");
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use linuxfs_core::BlockReader;
    use linuxfs_ext::ExtReadOnlyBackend;
    use linuxfs_storage::RawImageReader;
    use linuxfs_winfsp::{MountHost, native::NativeMountHost};
    use std::{
        env,
        io::{self, Read},
        sync::Arc,
    };

    let mut args = env::args_os().skip(1);
    let image = args
        .next()
        .ok_or("usage: mount_image <raw-image> <mount-point>")?;
    let mount_point = args
        .next()
        .ok_or("usage: mount_image <raw-image> <mount-point>")?;
    let _winfsp = winfsp::winfsp_init()?;
    let reader: Arc<dyn BlockReader> = Arc::new(RawImageReader::open(&image)?);
    let backend = ExtReadOnlyBackend::open(reader)?;
    let mut host = NativeMountHost::new(backend, "LinuxFS Manager", mount_point.to_string_lossy())?;
    host.mount()?;
    println!("Mounted read-only. Press Enter to unmount.");
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    host.unmount()?;
    Ok(())
}

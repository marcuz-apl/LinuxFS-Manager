//! Native WinFsp configuration kept behind the platform boundary.

use winfsp::{FspError, host::VolumeParams};

/// Build volume parameters that advertise and enforce read-only behavior.
///
/// This flag is defense in depth; callback-level mutation denial remains
/// mandatory and is implemented by the generic operation policy.
pub fn read_only_volume_params(filesystem_name: &str) -> VolumeParams {
    let mut params = VolumeParams::new();
    params
        .filesystem_name(filesystem_name)
        .read_only_volume(true)
        .case_preserved_names(true)
        .unicode_on_disk(true)
        .persistent_acls(false)
        .named_streams(false)
        .extended_attributes(false)
        .reparse_points(false);
    params
}

/// Return WinFsp's access-denied result for every source-mutating callback.
pub fn deny_mutation() -> winfsp::Result<()> {
    Err(FspError::WIN32(5))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_configuration_is_constructible() {
        let _params = read_only_volume_params("LinuxFS Manager");
        assert!(deny_mutation().is_err());
    }
}

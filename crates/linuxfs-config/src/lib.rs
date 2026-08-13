use linuxfs_core::{Error, ErrorCategory, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const CURRENT_CONFIG_VERSION: u32 = 1;
pub const MAX_RECENT_IMAGES: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub config_version: u32,
    #[serde(default)]
    pub preferred_drive_letter: Option<String>,
    #[serde(default)]
    pub ui_language: Option<String>,
    #[serde(default)]
    pub recent_images: Vec<String>,
    #[serde(default = "default_logging_enabled")]
    pub logging_enabled: bool,
}
fn default_logging_enabled() -> bool {
    true
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: CURRENT_CONFIG_VERSION,
            preferred_drive_letter: None,
            ui_language: None,
            recent_images: Vec::new(),
            logging_enabled: true,
        }
    }
}
impl AppConfig {
    pub fn bounded(mut self) -> Self {
        self.recent_images.truncate(MAX_RECENT_IMAGES);
        self
    }
    pub fn record_recent_image(&mut self, path: impl Into<String>) {
        let path = path.into();
        self.recent_images.retain(|p| p != &path);
        self.recent_images.insert(0, path);
        self.recent_images.truncate(MAX_RECENT_IMAGES);
    }
}
pub struct ConfigStore {
    path: PathBuf,
}
impl ConfigStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn load(&self) -> Result<AppConfig> {
        let text = match fs::read_to_string(&self.path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(AppConfig::default()),
            Err(e) => return Err(config_error("cannot read configuration", e)),
        };
        let config: AppConfig =
            toml::from_str(&text).map_err(|e| config_error("configuration is malformed", e))?;
        if config.config_version != CURRENT_CONFIG_VERSION {
            return Err(Error::new(
                ErrorCategory::Configuration,
                format!(
                    "unsupported configuration version {}",
                    config.config_version
                ),
            ));
        }
        Ok(config.bounded())
    }
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        let text = toml::to_string_pretty(&config.clone().bounded())
            .map_err(|e| config_error("cannot serialize configuration", e))?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|e| config_error("cannot create configuration directory", e))?;
        let temporary = self.path.with_extension("toml.tmp");
        fs::write(&temporary, text).map_err(|e| config_error("cannot write configuration", e))?;
        if let Err(e) = replace_file(&temporary, &self.path) {
            let _ = fs::remove_file(&temporary);
            return Err(config_error("cannot atomically replace configuration", e));
        }
        Ok(())
    }
}

/// Writes diagnostic text through the same atomic replacement path as config.
pub fn write_text_atomic(path: impl AsRef<Path>, text: &str) -> Result<()> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|e| config_error("cannot create diagnostic directory", e))?;
    let temporary = path.with_extension("log.tmp");
    fs::write(&temporary, text).map_err(|e| config_error("cannot write diagnostics", e))?;
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(config_error("cannot atomically replace diagnostics", error));
    }
    Ok(())
}

#[allow(unsafe_code)]
fn replace_file(temporary: &Path, destination: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
        let wide = |p: &Path| {
            OsStr::new(p)
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let temporary = wide(temporary);
        let destination = wide(destination); // SAFETY: buffers are owned, NUL-terminated UTF-16 paths valid for this call.
        let result = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                temporary.as_ptr(),
                destination.as_ptr(),
                0x1 | 0x8,
            )
        };
        if result == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(temporary, destination)
    }
}
fn config_error(message: &str, source: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::with_source(ErrorCategory::Configuration, message, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    fn temp_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("linuxfs-manager-config-{nonce}.toml"))
    }
    #[test]
    fn absent_config_uses_safe_defaults() {
        assert_eq!(
            ConfigStore::new(temp_path()).load().expect("defaults"),
            AppConfig::default()
        );
    }
    #[test]
    fn config_round_trips_and_recent_images_are_bounded() {
        let path = temp_path();
        let store = ConfigStore::new(&path);
        let mut config = AppConfig {
            preferred_drive_letter: Some("L".into()),
            ..Default::default()
        };
        for i in 0..(MAX_RECENT_IMAGES + 2) {
            config.record_recent_image(format!("image-{i}.raw"));
        }
        store.save(&config).expect("save");
        assert_eq!(store.load().expect("load"), config.bounded());
        let _ = fs::remove_file(path);
    }
    #[test]
    fn config_round_trips_an_optional_ui_language() {
        let path = temp_path();
        let store = ConfigStore::new(&path);
        let config = AppConfig {
            ui_language: Some("ja-JP".into()),
            ..Default::default()
        };

        store.save(&config).expect("save");
        assert_eq!(
            store.load().expect("load").ui_language,
            Some("ja-JP".into())
        );
        let _ = fs::remove_file(path);
    }
    #[test]
    fn malformed_and_unknown_versions_are_configuration_errors() {
        let path = temp_path();
        fs::write(&path, "not = [valid").expect("write malformed");
        let store = ConfigStore::new(&path);
        assert_eq!(
            store.load().expect_err("malformed").category(),
            ErrorCategory::Configuration
        );
        fs::write(
            &path,
            "config_version = 99
",
        )
        .expect("write version");
        assert_eq!(
            store.load().expect_err("version").category(),
            ErrorCategory::Configuration
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn atomic_text_write_replaces_existing_file() {
        let path = temp_path();
        fs::write(&path, "old").expect("seed file");
        write_text_atomic(&path, "new diagnostics").expect("atomic text write");
        assert_eq!(
            fs::read_to_string(&path).expect("read file"),
            "new diagnostics"
        );
        let _ = fs::remove_file(path);
    }
}

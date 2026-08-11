use std::path::PathBuf;

use linuxfs_config::{AppConfig, ConfigStore};
use linuxfs_core::{Error, ErrorCategory, Result};

pub fn config_store() -> ConfigStore {
    ConfigStore::new(config_path())
}

pub fn load_config() -> Result<AppConfig> {
    config_store().load()
}

pub fn initialize_logging(config: &AppConfig) -> Result<()> {
    if !config.logging_enabled {
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .map_err(|error| {
            Error::new(
                ErrorCategory::Configuration,
                format!("cannot initialize logging: {error}"),
            )
        })
}

fn config_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            return PathBuf::from(app_data)
                .join("LinuxFS Manager")
                .join("config.toml");
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(config_home)
                .join("LinuxFS Manager")
                .join("config.toml");
        }
    }
    std::env::temp_dir()
        .join("LinuxFS Manager")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_is_named_for_the_application() {
        assert_eq!(
            config_store()
                .path()
                .file_name()
                .and_then(|name| name.to_str()),
            Some("config.toml")
        );
        assert!(
            config_store()
                .path()
                .to_string_lossy()
                .contains("LinuxFS Manager")
        );
    }

    #[test]
    fn disabled_logging_is_a_safe_noop() {
        initialize_logging(&AppConfig {
            logging_enabled: false,
            ..Default::default()
        })
        .expect("disabled logging");
    }
}

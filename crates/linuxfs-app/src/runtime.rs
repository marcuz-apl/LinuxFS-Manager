use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

use linuxfs_config::{AppConfig, ConfigStore};
use linuxfs_core::{Error, ErrorCategory, Result};

/// A single application operation running away from the Slint/UI thread.
///
/// The operation is intentionally generic: storage discovery, image probing,
/// and mounting can each provide their existing synchronous implementation as
/// the closure without changing those APIs.
pub struct BackgroundOperation<T> {
    receiver: Receiver<Result<T>>,
    worker: Option<JoinHandle<()>>,
}

impl<T> BackgroundOperation<T> {
    /// Returns the result when the worker has completed, or `None` while it is
    /// still running. This method never blocks.
    pub fn try_receive(&mut self) -> Option<Result<T>> {
        match self.receiver.try_recv() {
            Ok(result) => {
                self.finish_worker();
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(Error::new(
                ErrorCategory::Internal,
                "background operation stopped without reporting a result",
            ))),
        }
    }

    /// Waits for the operation to complete and returns its result.
    pub fn wait(mut self) -> Result<T> {
        let result = self.receiver.recv().map_err(|_| {
            Error::new(
                ErrorCategory::Internal,
                "background operation stopped without reporting a result",
            )
        })?;
        self.finish_worker();
        result
    }

    fn finish_worker(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl<T> Drop for BackgroundOperation<T> {
    fn drop(&mut self) {
        self.finish_worker();
    }
}

/// Starts one bounded application operation on a worker thread.
pub fn spawn_background<T, F>(operation: F) -> BackgroundOperation<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        let _ = sender.send(operation());
    });
    BackgroundOperation {
        receiver,
        worker: Some(worker),
    }
}

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

/// Returns the diagnostic-only record path for the latest live WinFsp assessment.
pub fn winfsp_status_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("LinuxFS Manager")
                .join("winfsp-status.toml");
        }
    }
    std::env::temp_dir()
        .join("LinuxFS Manager")
        .join("winfsp-status.toml")
}

/// Writes a diagnostic snapshot of a live WinFsp assessment. The record is
/// never read to authorize mounting or bypass a fresh prerequisite check.
pub fn record_winfsp_assessment(assessment: &linuxfs_winfsp::WinFspAssessment) -> Result<PathBuf> {
    let path = winfsp_status_path();
    record_winfsp_assessment_at_path(&path, assessment)?;
    Ok(path)
}

fn record_winfsp_assessment_at_path(
    path: &std::path::Path,
    assessment: &linuxfs_winfsp::WinFspAssessment,
) -> Result<()> {
    use linuxfs_winfsp::{WinFspLauncherStatus, WinFspRequirement};
    use std::time::{SystemTime, UNIX_EPOCH};

    let requirement = match assessment.requirement() {
        WinFspRequirement::Ready => "ready",
        WinFspRequirement::UnsupportedPlatform => "unsupported_platform",
        WinFspRequirement::InstallationNotRegistered => "installation_not_registered",
        WinFspRequirement::RuntimeDllMissing => "runtime_dll_missing",
        WinFspRequirement::LauncherNotInstalled => "launcher_not_installed",
        WinFspRequirement::LauncherNotRunning => "launcher_not_running",
        WinFspRequirement::LauncherStatusUnavailable => "launcher_status_unavailable",
        WinFspRequirement::RuntimeInitializationFailed => "runtime_initialization_failed",
    };
    let launcher_service = match assessment.launcher_status() {
        WinFspLauncherStatus::NotInstalled => "not_installed",
        WinFspLauncherStatus::Stopped => "stopped",
        WinFspLauncherStatus::Running => "running",
        WinFspLauncherStatus::QueryFailed => "query_failed",
        WinFspLauncherStatus::UnsupportedPlatform => "unsupported_platform",
    };
    let checked_at_utc_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let content = format!(
        "status_version = 1\nchecked_at_utc_unix_seconds = {checked_at_utc_unix_seconds}\nstatus = \"{requirement}\"\ninstallation_registered = {}\nruntime_dll_present = {}\nlauncher_service = \"{launcher_service}\"\nruntime_initialized = {}\n",
        assessment.installation_registered(),
        assessment.runtime_dll_present(),
        assessment.runtime_initialized(),
    );
    linuxfs_config::write_text_atomic(path, &content)
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
mod background_tests {
    use super::*;

    #[test]
    fn background_operation_reports_completion_without_blocking_poll() {
        let mut operation = spawn_background(|| Ok::<_, Error>(42_u32));
        let result = loop {
            if let Some(result) = operation.try_receive() {
                break result;
            }
            thread::yield_now();
        };
        assert_eq!(result.expect("worker completion"), 42);
    }

    #[test]
    fn background_operation_reports_worker_errors() {
        let error = Error::new(ErrorCategory::StorageAccess, "probe failed");
        let result = spawn_background(move || Err::<(), _>(error)).wait();
        assert_eq!(
            result.expect_err("worker error").category(),
            ErrorCategory::StorageAccess
        );
    }

    #[test]
    fn try_receive_returns_none_while_worker_is_running() {
        let (release_sender, release_receiver) = mpsc::channel();
        let mut operation = spawn_background(move || {
            release_receiver.recv().expect("release signal");
            Ok::<_, Error>(())
        });
        assert!(operation.try_receive().is_none());
        release_sender.send(()).expect("release worker");
        assert!(operation.wait().is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linuxfs_winfsp::{WinFspAssessment, WinFspLauncherStatus};

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

    #[test]
    fn winfsp_status_record_is_versioned_and_diagnostic() {
        let directory = std::env::temp_dir().join(format!(
            "linuxfs-manager-winfsp-status-test-{}",
            std::process::id()
        ));
        let path = directory.join("winfsp-status.toml");
        let assessment =
            WinFspAssessment::from_checks(true, true, WinFspLauncherStatus::Running, true);

        record_winfsp_assessment_at_path(&path, &assessment).expect("write status record");

        let content = std::fs::read_to_string(&path).expect("read status record");
        assert!(content.contains("status_version = 1"));
        assert!(content.contains("status = \"ready\""));
        assert!(content.contains("launcher_service = \"running\""));
        std::fs::remove_dir_all(directory).expect("remove test directory");
    }
}

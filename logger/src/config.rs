use config_loader::Config as LoaderConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Configuration for the logger module.
///
/// It is loaded from a YAML file using the [`config_loader`] crate.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether to use colored output on the console.
    pub colorful: bool,
    /// Base filename of the log file.
    pub filename: String,
    /// Maximum size of a log file in megabytes before rotation.
    pub max_size: u64,
    /// Maximum number of old log files to retain. `0` keeps all old files.
    pub max_backups: u32,
    /// Number of days after which a log file is rotated.
    pub rotate_log_after_days: u32,
    /// Whether to gzip-compress rotated log files.
    pub compress: bool,
    /// Log targets: `console` and/or `file`.
    pub targets: Vec<String>,
    /// Per-name log levels, with the root level stored under `default`.
    pub levels: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colorful: true,
            filename: "app.log".to_string(),
            max_size: 10,
            max_backups: 0,
            rotate_log_after_days: 1,
            compress: true,
            targets: vec!["console".to_string()],
            levels: HashMap::from([("default".to_string(), "debug".to_string())]),
        }
    }
}

impl Config {
    /// Loads the logger configuration from a YAML file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, config_loader::ConfigError> {
        config_loader::load_from_yaml(path)
    }
}

impl LoaderConfig for Config {
    fn basic_check(&self) -> Result<(), String> {
        for target in &self.targets {
            if target != "console" && target != "file" {
                return Err(format!(
                    "invalid logging target {target:?} (must be 'console' or 'file')"
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    struct TempFile {
        _dir: TempDir,
        path: std::path::PathBuf,
    }

    impl TempFile {
        fn new(name: &str, content: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(name);
            fs::write(&path, content).unwrap();
            Self { _dir: dir, path }
        }
    }

    #[test]
    fn loads_defaults_for_missing_fields() {
        let file = TempFile::new("config.yaml", "targets:\n  - console\n");
        let config = Config::load(file.path).unwrap();

        assert!(config.colorful);
        assert_eq!(config.filename, "app.log");
        assert_eq!(config.max_size, 10);
        assert_eq!(config.max_backups, 0);
        assert_eq!(config.rotate_log_after_days, 1);
        assert!(config.compress);
        assert_eq!(config.targets, vec!["console"]);
        assert_eq!(
            config.levels.get("default").map(String::as_str),
            Some("debug")
        );
    }

    #[test]
    fn loads_full_config() {
        let file = TempFile::new(
            "config.yaml",
            r#"
colorful: false
filename: test.log
max_size: 5
max_backups: 3
rotate_log_after_days: 7
compress: false
targets:
  - console
  - file
levels:
  default: info
  network: warn
"#,
        );
        let config = Config::load(file.path).unwrap();

        assert!(!config.colorful);
        assert_eq!(config.filename, "test.log");
        assert_eq!(config.max_size, 5);
        assert_eq!(config.max_backups, 3);
        assert_eq!(config.rotate_log_after_days, 7);
        assert!(!config.compress);
        assert_eq!(config.targets, vec!["console", "file"]);
        assert_eq!(
            config.levels.get("default").map(String::as_str),
            Some("info")
        );
        assert_eq!(
            config.levels.get("network").map(String::as_str),
            Some("warn")
        );
    }

    #[test]
    fn rejects_invalid_targets() {
        let file = TempFile::new(
            "config.yaml",
            r#"
targets:
  - console
  - database
"#,
        );
        let result = Config::load(file.path);

        assert!(matches!(
            result,
            Err(config_loader::ConfigError::Validation { .. })
        ));
    }
}

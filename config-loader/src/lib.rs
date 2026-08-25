//! A minimal configuration-loading utility.
//!
//! This crate provides a [`Config`] trait and loaders for YAML and TOML files.
//! It mirrors the Go `config` package: a configuration file is parsed into a
//! type implementing [`Config`], then [`Config::override_values`] is applied,
//! and finally [`Config::basic_check`] validates the result.
//!
//! # Features
//!
//! - `yaml` (enabled by default): enables `load_from_yaml` and related APIs.
//! - `toml`: enables `load_from_toml` and related APIs.
//!
//! # Example
//!
//! ```rust,no_run
//! use config_loader::{load_from_yaml, Config};
//! use serde::Deserialize;
//!
//! #[derive(Debug, Default, Deserialize)]
//! struct AppConfig {
//!     host: String,
//!     port: u16,
//! }
//!
//! impl Config for AppConfig {
//!     fn basic_check(&self) -> Result<(), String> {
//!         if self.port == 0 {
//!             return Err("port must not be 0".to_string());
//!         }
//!         Ok(())
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let cfg: AppConfig = load_from_yaml("config.yaml")?;
//! # Ok(())
//! # }
//! ```

use serde::de::DeserializeOwned;
use std::error::Error as StdError;
#[cfg(any(feature = "yaml", feature = "toml"))]
use std::fs;
use std::io;
#[cfg(any(feature = "yaml", feature = "toml"))]
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur while loading a configuration file.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The configuration file could not be read.
    #[error("failed to read config file `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The configuration file could not be parsed.
    #[error("failed to parse config file `{path}`: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    /// Strict mode was enabled and unknown fields were found.
    #[error("unknown config field(s) in `{path}`: {fields}")]
    UnknownFields { path: PathBuf, fields: String },

    /// The configuration failed validation.
    #[error("config validation failed: {message}")]
    Validation { message: String },
}

/// Options that control how a configuration file is loaded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LoadOptions {
    strict: bool,
}

impl LoadOptions {
    /// Creates the default load options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Configures whether unknown fields are rejected.
    pub fn strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Returns whether strict mode is enabled.
    pub fn is_strict(&self) -> bool {
        self.strict
    }
}

/// Trait implemented by configuration types that can be loaded from a file.
pub trait Config: DeserializeOwned + Default {
    /// Applies overrides after parsing, for example from environment variables.
    fn override_values(&mut self) {}

    /// Validates the configuration after parsing and overrides.
    fn basic_check(&self) -> Result<(), String> {
        Ok(())
    }

    /// Loads `Self` from a YAML file.
    #[cfg(feature = "yaml")]
    fn load_from_yaml<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>
    where
        Self: Sized,
    {
        load_from_yaml(path)
    }

    /// Loads `Self` from a YAML file with the given options.
    #[cfg(feature = "yaml")]
    fn load_from_yaml_with_options<P: AsRef<Path>>(
        path: P,
        options: LoadOptions,
    ) -> Result<Self, ConfigError>
    where
        Self: Sized,
    {
        load_from_yaml_with_options(path, options)
    }

    /// Loads `Self` from a TOML file.
    #[cfg(feature = "toml")]
    fn load_from_toml<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError>
    where
        Self: Sized,
    {
        load_from_toml(path)
    }

    /// Loads `Self` from a TOML file with the given options.
    #[cfg(feature = "toml")]
    fn load_from_toml_with_options<P: AsRef<Path>>(
        path: P,
        options: LoadOptions,
    ) -> Result<Self, ConfigError>
    where
        Self: Sized,
    {
        load_from_toml_with_options(path, options)
    }
}

/// Loads `T` from the YAML file at `path`.
///
/// See [`load_from_yaml_with_options`] to enable strict mode.
#[cfg(feature = "yaml")]
pub fn load_from_yaml<P: AsRef<Path>, T: Config>(path: P) -> Result<T, ConfigError> {
    load_from_yaml_with_options(path, LoadOptions::default())
}

/// Loads `T` from the YAML file at `path` with the given [`LoadOptions`].
#[cfg(feature = "yaml")]
pub fn load_from_yaml_with_options<P: AsRef<Path>, T: Config>(
    path: P,
    options: LoadOptions,
) -> Result<T, ConfigError> {
    let path = path.as_ref();
    load(path, |content| {
        let deserializer = yaml_serde::Deserializer::from_str(content);
        deserialize::<_, T>(deserializer, path, options)
    })
}

/// Loads `T` from the TOML file at `path`.
///
/// See [`load_from_toml_with_options`] to enable strict mode.
#[cfg(feature = "toml")]
pub fn load_from_toml<P: AsRef<Path>, T: Config>(path: P) -> Result<T, ConfigError> {
    load_from_toml_with_options(path, LoadOptions::default())
}

/// Loads `T` from the TOML file at `path` with the given [`LoadOptions`].
#[cfg(feature = "toml")]
pub fn load_from_toml_with_options<P: AsRef<Path>, T: Config>(
    path: P,
    options: LoadOptions,
) -> Result<T, ConfigError> {
    let path = path.as_ref();
    load(path, |content| {
        let deserializer =
            toml::Deserializer::parse(content).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source: Box::new(source),
            })?;
        deserialize::<_, T>(deserializer, path, options)
    })
}

#[cfg(any(feature = "yaml", feature = "toml"))]
fn load<T, F>(path: &Path, parse: F) -> Result<T, ConfigError>
where
    T: Config,
    F: FnOnce(&str) -> Result<T, ConfigError>,
{
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let mut config = parse(&content)?;
    config.override_values();
    config
        .basic_check()
        .map_err(|message| ConfigError::Validation { message })?;

    Ok(config)
}

#[cfg(any(feature = "yaml", feature = "toml"))]
fn deserialize<'de, D, T>(
    deserializer: D,
    path: &Path,
    options: LoadOptions,
) -> Result<T, ConfigError>
where
    D: serde::Deserializer<'de>,
    D::Error: StdError + Send + Sync + 'static,
    T: DeserializeOwned,
{
    if options.is_strict() {
        let mut fields = Vec::new();
        let config = serde_ignored::deserialize(deserializer, |path| {
            fields.push(path.to_string());
        })
        .map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;

        if !fields.is_empty() {
            return Err(ConfigError::UnknownFields {
                path: path.to_path_buf(),
                fields: fields.join(", "),
            });
        }

        Ok(config)
    } else {
        T::deserialize(deserializer).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;
    use tempfile::TempDir;

    #[derive(Debug, Default, Deserialize, PartialEq, Eq)]
    #[serde(default)]
    struct TestConfig {
        key1: String,
        key2: String,
    }

    impl Config for TestConfig {
        fn override_values(&mut self) {
            if let Ok(value) = std::env::var("YAML_KEY2_OVERRIDE")
                && !value.is_empty()
            {
                self.key2 = value;
            }

            if let Ok(value) = std::env::var("TOML_KEY2_OVERRIDE")
                && !value.is_empty()
            {
                self.key2 = value;
            }
        }

        fn basic_check(&self) -> Result<(), String> {
            if self.key1.is_empty() || self.key2.is_empty() {
                return Err("key1 and key2 must not be empty".to_string());
            }

            Ok(())
        }
    }

    struct TempFile {
        _dir: TempDir,
        path: PathBuf,
    }

    impl TempFile {
        fn new(name: &str, content: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(name);
            fs::write(&path, content).unwrap();
            Self { _dir: dir, path }
        }
    }

    #[cfg(feature = "yaml")]
    #[serial_test::serial]
    #[test]
    fn yaml_successful_load() {
        let file = TempFile::new("config.yaml", "key1: value1\nkey2: value2\n");
        let cfg: TestConfig = load_from_yaml(file.path).unwrap();

        assert_eq!(
            cfg,
            TestConfig {
                key1: "value1".to_string(),
                key2: "value2".to_string(),
            }
        );
    }

    #[cfg(feature = "yaml")]
    #[serial_test::serial]
    #[test]
    fn yaml_strict_rejects_unknown_fields() {
        let file = TempFile::new(
            "config.yaml",
            "key1: value1\nkey2: value2\nkey_unknown: value_unknown\n",
        );

        let result: Result<TestConfig, _> =
            load_from_yaml_with_options(&file.path, LoadOptions::new().strict(true));
        assert!(matches!(result, Err(ConfigError::UnknownFields { .. })));

        let cfg: TestConfig =
            load_from_yaml_with_options(&file.path, LoadOptions::default()).unwrap();
        assert_eq!(cfg.key2, "value2");
    }

    #[cfg(feature = "yaml")]
    #[serial_test::serial]
    #[test]
    fn yaml_basic_check_fails() {
        let file = TempFile::new("config.yaml", "key1: value1\n");
        let result: Result<TestConfig, _> = load_from_yaml(file.path);

        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    #[cfg(feature = "yaml")]
    #[serial_test::serial]
    #[test]
    fn yaml_override_values() {
        // SAFETY: uses a dedicated env var that no other test reads.
        unsafe { std::env::set_var("YAML_KEY2_OVERRIDE", "overridden2") };

        let file = TempFile::new("config.yaml", "key1: value1\nkey2: value2\n");
        let cfg: TestConfig = load_from_yaml(file.path).unwrap();

        assert_eq!(cfg.key2, "overridden2");

        // SAFETY: removes the dedicated env var created by this test.
        unsafe { std::env::remove_var("YAML_KEY2_OVERRIDE") };
    }

    #[cfg(feature = "toml")]
    #[serial_test::serial]
    #[test]
    fn toml_successful_load() {
        let file = TempFile::new("config.toml", "key1 = 'value1'\nkey2 = 'value2'\n");
        let cfg: TestConfig = load_from_toml(file.path).unwrap();

        assert_eq!(
            cfg,
            TestConfig {
                key1: "value1".to_string(),
                key2: "value2".to_string(),
            }
        );
    }

    #[cfg(feature = "toml")]
    #[serial_test::serial]
    #[test]
    fn toml_strict_rejects_unknown_fields() {
        let file = TempFile::new(
            "config.toml",
            "key1 = 'value1'\nkey2 = 'value2'\nkey_unknown = 'value_unknown'\n",
        );

        let result: Result<TestConfig, _> =
            load_from_toml_with_options(&file.path, LoadOptions::new().strict(true));
        assert!(matches!(result, Err(ConfigError::UnknownFields { .. })));

        let cfg: TestConfig =
            load_from_toml_with_options(&file.path, LoadOptions::default()).unwrap();
        assert_eq!(cfg.key2, "value2");
    }

    #[cfg(feature = "toml")]
    #[serial_test::serial]
    #[test]
    fn toml_basic_check_fails() {
        let file = TempFile::new("config.toml", "key1 = 'value1'\n");
        let result: Result<TestConfig, _> = load_from_toml(file.path);

        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    #[cfg(feature = "toml")]
    #[serial_test::serial]
    #[test]
    fn toml_override_values() {
        // SAFETY: uses a dedicated env var that no other test reads.
        unsafe { std::env::set_var("TOML_KEY2_OVERRIDE", "overridden2") };

        let file = TempFile::new("config.toml", "key1 = 'value1'\nkey2 = 'value2'\n");
        let cfg: TestConfig = load_from_toml(file.path).unwrap();

        assert_eq!(cfg.key2, "overridden2");

        // SAFETY: removes the dedicated env var created by this test.
        unsafe { std::env::remove_var("TOML_KEY2_OVERRIDE") };
    }
}

pub mod config;

pub use config::Config;

use anyhow::Context;
use config::Config as LoggerConfig;
use log::LevelFilter;
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            LogFile, RollingFileAppender,
            policy::compound::{
                CompoundPolicy,
                roll::fixed_window::FixedWindowRoller,
                trigger::{
                    Trigger,
                    size::SizeTrigger,
                    time::{TimeTrigger, TimeTriggerConfig, TimeTriggerInterval},
                },
            },
        },
    },
    config::{Appender, Config as Log4rsConfig, Root},
    encode::{Encode, json::JsonEncoder, pattern::PatternEncoder},
};
use std::path::Path;
use std::str::FromStr;

/// A trigger that rolls the log file when either the size or the time
/// condition is met.
#[derive(Debug)]
struct SizeOrTimeTrigger {
    size: SizeTrigger,
    time: TimeTrigger,
}

impl Trigger for SizeOrTimeTrigger {
    fn trigger(&self, file: &LogFile) -> anyhow::Result<bool> {
        Ok(self.size.trigger(file)? || self.time.trigger(file)?)
    }

    fn is_pre_process(&self) -> bool {
        self.size.is_pre_process() || self.time.is_pre_process()
    }
}

/// Builds a rolling file appender with size and age based rotation.
fn build_file_appender(config: &LoggerConfig) -> anyhow::Result<RollingFileAppender> {
    let max_size = if config.max_size > 0 {
        config.max_size.saturating_mul(1024 * 1024)
    } else {
        10 * 1024 * 1024
    };
    let size_trigger = SizeTrigger::new(max_size);

    let days = if config.rotate_log_after_days > 0 {
        config.rotate_log_after_days
    } else {
        1
    };
    let time_trigger = TimeTrigger::new(TimeTriggerConfig {
        interval: TimeTriggerInterval::Day(days as i64),
        ..Default::default()
    });

    let trigger = SizeOrTimeTrigger {
        size: size_trigger,
        time: time_trigger,
    };

    // log4rs uses a fixed window; 0 means "delete the rolled file". Go's
    // lumberjack uses 0 to mean "retain all", so approximate that with a large
    // window.
    let count = if config.max_backups == 0 {
        10_000
    } else {
        config.max_backups
    };

    let pattern = if config.compress {
        format!("{}.{{}}.gz", config.filename)
    } else {
        format!("{}.{{}}", config.filename)
    };
    let roller = FixedWindowRoller::builder().build(&pattern, count)?;

    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    let appender = RollingFileAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build(&config.filename, Box::new(policy))?;

    Ok(appender)
}

/// Builds a console appender using a plain or colored pattern.
fn build_console_appender(config: &LoggerConfig) -> ConsoleAppender {
    let encoder: Box<dyn Encode> = if config.colorful {
        Box::new(PatternEncoder::new("{d} {h({l})} [{M}:{L}] - {m}{n}"))
    } else {
        Box::new(PatternEncoder::new("{d} {l} [{M}:{L}] - {m}{n}"))
    };

    ConsoleAppender::builder().encoder(encoder).build()
}

/// Initializes the global logger from the given configuration.
pub fn init_logger(config: &LoggerConfig) -> anyhow::Result<()> {
    let mut root_builder = Root::builder();
    let mut builder = Log4rsConfig::builder();

    if config.targets.iter().any(|target| target == "file") {
        let appender = build_file_appender(config)?;
        builder = builder.appender(Appender::builder().build("file", Box::new(appender)));
        root_builder = root_builder.appender("file");
    }

    if config.targets.iter().any(|target| target == "console") {
        let appender = build_console_appender(config);
        builder = builder.appender(Appender::builder().build("console", Box::new(appender)));
        root_builder = root_builder.appender("console");
    }

    let level = config
        .levels
        .get("default")
        .map(|value| LevelFilter::from_str(value))
        .transpose()
        .context("invalid default log level")?
        .unwrap_or(LevelFilter::Debug);

    let log4rs = builder.build(root_builder.build(level))?;
    log4rs::init_config(log4rs)?;

    Ok(())
}

/// Loads the logger configuration from a YAML file and initializes the global
/// logger.
pub fn init_logger_from_yaml<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    let config = LoggerConfig::load(path)?;
    init_logger(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rolling_file_appender() {
        let dir = tempfile::tempdir().unwrap();
        let config = LoggerConfig {
            targets: vec!["file".to_string()],
            filename: dir.path().join("test.log").to_string_lossy().to_string(),
            ..Default::default()
        };

        let _appender = build_file_appender(&config).unwrap();

        assert!(dir.path().join("test.log").exists());
    }
}

pub mod config;

use crate::config::Config;
use log::LevelFilter;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    config::{Appender, Config as Log4rsConfig, Logger, Root},
    encode::{json::JsonEncoder, pattern::PatternEncoder},
};
use std::str::FromStr;

pub fn init_logger(config: &Config) {
    let level = LevelFilter::from_str(&config.level).unwrap();
    let mut root_builder = Root::builder();
    let mut builder = Log4rsConfig::builder();
    if !config.file.is_empty() {
        let logfile = FileAppender::builder()
            .encoder(Box::new(JsonEncoder::new()))
            .build(config.file.clone())
            .unwrap();

        builder = builder
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .logger(
                Logger::builder()
                    .appender("logfile")
                    .additive(false)
                    .build("file", level),
            );
        root_builder = root_builder.appender("logfile");
    }

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d} {h({l})} [{M}:{L}] - {m}{n}",
        )))
        .build();

    let log4rs = builder
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(root_builder.appender("stdout").build(level))
        .unwrap();

    log4rs::init_config(log4rs).unwrap();
}

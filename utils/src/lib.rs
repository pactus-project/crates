use std::{panic, process};

pub fn value_or_error(value: &serde_json::Value, key: &str) -> anyhow::Result<serde_json::Value> {
    match value.get(key) {
        Some(value) => Ok(value.to_owned()),
        None => anyhow::bail!("Unable to find '{}' in '{}'", key, value),
    }
}

pub fn exit_on_panic() {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("Exiting On : {panic_info:#?}");
        orig_hook(panic_info);
        process::exit(1);
    }));
}

pub fn error_to_tonic_status(e: anyhow::Error) -> tonic::Status {
    tonic::Status::internal(e.to_string())
}

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    let log_filter = {
        if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        }
    };
    env_logger::builder()
        .filter_level(log_filter)
        .init();
    clientworks_lib::run().await
}

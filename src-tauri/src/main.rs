#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
    klasync_lib::api::start_server().await;
    klasync_lib::run();
}

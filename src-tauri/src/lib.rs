pub mod calendar;
pub mod commands;
pub mod db;
pub mod deps;
pub mod error;
pub mod repo;

pub use error::{GbError, GbResult};

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub mod db;
pub mod epub;
pub mod models;
pub mod schema;
pub mod seed;
pub mod utils;

mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    db::initialize_database();

    let mut connection = db::establish_connection();
    db::run_migrations(&mut connection).expect("Failed to run database migrations");
    // seed::create_directory();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::test::test])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::error::Error;

use diesel::{prelude::*, sqlite::Sqlite};
use directories::ProjectDirs;

pub fn establish_connection() -> SqliteConnection {
    if let Some(proj_dirs) = ProjectDirs::from("com", "seoayoon", "bookbug") {
        let database_url = proj_dirs
            .data_dir()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        SqliteConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
    } else {
        panic!("Could not determine project directories");
    }
}

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub fn run_migrations(
    connection: &mut impl MigrationHarness<Sqlite>,
) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // This will run the necessary migrations.
    //
    // See the documentation for `MigrationHarness` for
    // all available methods.
    connection.run_pending_migrations(MIGRATIONS)?;

    println!("Migrations ran successfully");
    Ok(())
}

pub fn initialize_database() {
    if let Some(proj_dirs) = ProjectDirs::from("com", "seoayoon", "bookbug") {
        let database_url = proj_dirs
            .data_dir()
            .join("database.db")
            .to_str()
            .unwrap()
            .to_string();

        std::fs::create_dir_all(proj_dirs.data_dir()).expect("Failed to create data directory");

        if !std::fs::exists(&database_url).unwrap_or(false) {
            std::fs::File::create(&database_url).expect("Failed to create database file");
        }
        
    }
}

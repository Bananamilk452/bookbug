use crate::models::Directory;
use diesel::prelude::*;
use uuid::Uuid;

pub fn create_directory() {
    use crate::schema::directories::dsl::*;

    let mut connection = crate::db::establish_connection();

    diesel::insert_into(directories)
        .values((
            id.eq(Uuid::now_v7().to_string()),
            path.eq("/home/seoayoon/pub"),
            created_at.eq(chrono::Utc::now().naive_utc()),
            updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(&mut connection)
        .expect("Error inserting new directory");

    let results = directories
        .limit(5)
        .load::<Directory>(&mut connection)
        .expect("Error loading directories");

    println!("Displaying {} directories", results.len());
    for directory in results {
        println!("{}: {}", directory.id, directory.path);
    }
}

use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::directories)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Directory {
    pub id: String,
    pub path: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

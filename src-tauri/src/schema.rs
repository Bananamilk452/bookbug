// @generated automatically by Diesel CLI.

diesel::table! {
    directories (id) {
        id -> Text,
        path -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

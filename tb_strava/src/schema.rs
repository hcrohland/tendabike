table! {
    activities (id) {
        id -> Int8,
        tendabike_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    gears (id) {
        id -> Text,
        tendabike_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    users (id) {
        id -> Int4,
        tendabike_id -> Int4,
        last_activity -> Int8,
        access_token -> Text,
        expires_at -> Int8,
        refresh_token -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    activities,
    gears,
    users,
);

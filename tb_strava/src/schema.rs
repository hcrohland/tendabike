table! {
    activities (id) {
        id -> Int8,
        tendabike_id -> Nullable<Int4>,
        user_id -> Int4,
        gear_id -> Nullable<Text>,
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
        tendabike_id -> Nullable<Int4>,
        last_activity -> Nullable<Int8>,
        access_token -> Text,
        expires_at -> Int8,
        refresh_token -> Text,
    }
}

joinable!(activities -> gears (gear_id));
joinable!(activities -> users (user_id));
joinable!(gears -> users (user_id));

allow_tables_to_appear_in_same_query!(
    activities,
    gears,
    users,
);

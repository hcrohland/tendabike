use diesel::prelude::*;

table! {
    strava_activities (id) {
        id -> Int8,
        tendabike_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    strava_events (id) {
        id -> Nullable<Int4>,
        object_type -> Text,
        object_id -> Int8,
        aspect_type -> Text,
        updates -> Text,
        owner_id -> Int4,
        subscription_id -> Int4,
        event_time -> Int8,
    }
}

table! {
    strava_users (id) {
        id -> Int4,
        tendabike_id -> Int4,
        last_activity -> Int8,
        refresh_token -> Nullable<Text>,
    }
}

table! {
    users (id) {
        id -> Int4,
        name -> Text,
        firstname -> Text,
        is_admin -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(strava_activities, strava_events, strava_users,);

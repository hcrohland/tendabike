table! {
    activities (id) {
        id -> Int4,
        user_id -> Int4,
        what -> Int4,
        name -> Text,
        start -> Timestamptz,
        duration -> Int4,
        time -> Nullable<Int4>,
        distance -> Nullable<Int4>,
        climb -> Nullable<Int4>,
        descend -> Nullable<Int4>,
        power -> Nullable<Int4>,
        gear -> Nullable<Int4>,
    }
}

table! {
    activity_types (id) {
        id -> Int4,
        name -> Text,
        gear -> Int4,
    }
}

table! {
    attachments (part_id, attached) {
        part_id -> Int4,
        attached -> Timestamptz,
        gear -> Int4,
        hook -> Int4,
        detached -> Nullable<Timestamptz>,
        count -> Int4,
        time -> Int4,
        distance -> Int4,
        climb -> Int4,
        descend -> Int4,
    }
}

table! {
    part_types (id) {
        id -> Int4,
        name -> Text,
        main -> Int4,
        hooks -> Array<Int4>,
        order -> Int4,
        group -> Nullable<Text>,
    }
}

table! {
    parts (id) {
        id -> Int4,
        owner -> Int4,
        what -> Int4,
        name -> Text,
        vendor -> Text,
        model -> Text,
        purchase -> Timestamptz,
        time -> Int4,
        distance -> Int4,
        climb -> Int4,
        descend -> Int4,
        count -> Int4,
        last_used -> Timestamptz,
        disposed_at -> Nullable<Timestamptz>,
    }
}

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
    strava_gears (id) {
        id -> Text,
        tendabike_id -> Int4,
        user_id -> Int4,
    }
}

table! {
    strava_users (id) {
        id -> Int4,
        tendabike_id -> Int4,
        last_activity -> Int8,
        access_token -> Text,
        expires_at -> Int8,
        refresh_token -> Text,
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

joinable!(activities -> activity_types (what));
joinable!(activity_types -> part_types (gear));
joinable!(attachments -> part_types (hook));
joinable!(parts -> part_types (what));

allow_tables_to_appear_in_same_query!(
    activities,
    activity_types,
    attachments,
    part_types,
    parts,
    strava_activities,
    strava_events,
    strava_gears,
    strava_users,
    users,
);

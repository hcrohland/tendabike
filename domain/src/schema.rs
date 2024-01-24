use diesel::prelude::*;

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
        detached -> Timestamptz,
        usage -> Uuid,
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
        last_used -> Timestamptz,
        disposed_at -> Nullable<Timestamptz>,
        usage -> Uuid,
    }
}

table! {
    usages(id) {
        id -> Uuid,
        time -> Int4,
        /// Usage distance
        distance -> Int4,
        /// Overall climbing
        climb -> Int4,
        /// Overall descending
        descend -> Int4,
        /// Overall descending
        power -> Int4,
        /// number of activities
        count -> Int4,
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
    users,
);

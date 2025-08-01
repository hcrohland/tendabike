use diesel::prelude::*;

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
        source -> Nullable<Text>
    }
}

table! {
    services(id) {
        id -> Uuid,
        part_id -> Int4,
        time -> Timestamptz,
        redone -> Timestamptz,
        name -> Text,
        notes -> Text,
        usage -> Uuid,
        successor -> Nullable<Uuid>,
        plans -> Array<Uuid>,
    }
}

table! {
    service_plans(id) {
        id -> Uuid,
        part -> Nullable<Int4>,
        what -> Int4,
        hook -> Nullable<Int4>,
        name -> Text,
        days -> Nullable<Int4>,
        hours -> Nullable<Int4>,
        km -> Nullable<Int4>,
        climb -> Nullable<Int4>,
        descend -> Nullable<Int4>,
        rides -> Nullable<Int4>,
        uid -> Nullable<Int4>,
        energy -> Nullable<Int4>,
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
        /// Overall energy
        energy -> Int4,
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

table! {
    activities(id) {
        user_id -> Int4,
        what -> Int4,
        name -> Text,
        start -> Timestamptz,
        duration -> Int4,
        time -> Nullable<Int4>,
        distance -> Nullable<Int4>,
        climb -> Nullable<Int4>,
        descend -> Nullable<Int4>,
        energy -> Nullable<Int4>,
        gear -> Nullable<Int4>,
        utc_offset -> Int4,
        id -> Int8,
    }
}
allow_tables_to_appear_in_same_query!(attachments, parts, users,);

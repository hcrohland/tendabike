table! {
    activities (id) {
        id -> Int4,
        user_id -> Int4,
        what -> Nullable<Int4>,
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
    part_types (id) {
        id -> Int4,
        name -> Text,
        parts -> Array<Int4>,
        main -> Bool,
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
        attached_to -> Nullable<Int4>,
        count -> Int4,
    }
}

joinable!(activities -> activity_types (what));
joinable!(activity_types -> part_types (gear));
joinable!(parts -> part_types (what));

allow_tables_to_appear_in_same_query!(
    activities,
    activity_types,
    part_types,
    parts,
);

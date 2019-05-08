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
        user_id -> Int4,
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
    }
}

table! {
    greetings (id) {
        id -> Int4,
        text -> Text,
    }
}

joinable!(parts -> part_types (what));

allow_tables_to_appear_in_same_query!(
    part_types,
    parts,
    greetings,
);

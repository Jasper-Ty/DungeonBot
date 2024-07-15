// @generated automatically by Diesel CLI.

diesel::table! {
    state (key) {
        key -> Nullable<Text>,
        value -> Text,
    }
}

diesel::table! {
    users (id) {
        id -> BigInt,
        points -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    state,
    users,
);

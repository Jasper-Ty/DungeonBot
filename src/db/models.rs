use diesel::prelude::*;

use crate::schema::users;

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i64,
    pub points: i32
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub id: i64,
}

use crate::schema::state;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = state)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct StateVar {
    pub key: String,
    pub value: String,
}


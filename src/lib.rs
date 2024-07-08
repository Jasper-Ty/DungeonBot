pub mod models;
pub mod schema;

pub mod commands;
pub mod lastmessage;

use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic! ("Error connecting to {}", database_url))
}

use models::User;

pub fn new_user(conn: &mut SqliteConnection, user_id: u64) -> Option<User> {
    use schema::users;

    let new_user = models::NewUser { id: user_id as i64 };

    diesel::insert_into(users::table)
        .values(&new_user)
        .on_conflict(users::id)
        .do_nothing()
        .returning(models::User::as_returning())
        .get_result(conn)
        .optional()
        .ok()
        .flatten()
}

pub fn get_users(conn: &mut SqliteConnection) -> Vec<User> {
    use schema::users::dsl::*;

    users 
        .limit(10)
        .select(User::as_select())
        .load(conn)
        .expect("Error loading users")
}

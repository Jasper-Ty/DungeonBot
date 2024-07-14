pub mod db;

pub mod commands;
pub mod lastmessage;

use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

use db::{models, schema};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = core::result::Result<T, Error>;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic! ("Error connecting to {}", database_url))
}

use models::User;

pub fn new_user(conn: &mut SqliteConnection, user_id: u64) -> Option<User> {
    use schema::users::dsl::*;
    let user_id = user_id as i64;

    let new_user = models::NewUser { id: user_id };

    diesel::insert_into(users)
        .values(&new_user)
        .on_conflict(id)
        .do_nothing()
        .returning(models::User::as_returning())
        .get_result(conn)
        .optional()
        .ok()
        .flatten()
}

/// Gets the user `user_id`
pub fn get_user(conn: &mut SqliteConnection, user_id: u64) -> Option<User> {
    use schema::users::dsl::*;

    users
        .find(user_id as i64)
        .select(User::as_select())
        .first(conn)
        .optional()
        .expect("Error retrieving user")
}

/// Adds `points` points to user `user_id`.
pub fn add_points(conn: &mut SqliteConnection, user_id: u64, pts: i32) {
    use schema::users::dsl::*;
    let user_id = user_id as i64;

    diesel::update(users)
        .filter(id.eq(user_id))
        .set(points.eq(points + pts))
        .execute(conn)
        .expect("Error adding points");
}

pub fn top_users(conn: &mut SqliteConnection, lim: i64, off: i64) -> Vec<User> {
    use schema::users::dsl::*;

    users 
        .limit(lim)
        .offset(off)
        .order_by(points.desc())
        .select((id, points))
        .load(conn)
        .expect("Error loading users")
}

/// Convenience function for getting a type whose underlying
/// id data type is a snowflake (e.g a user id).
pub fn env_snowflake<T: From<u64>> (key: &str) -> Result<T> {
    Ok(T::from(
        env::var(key)?
        .parse::<u64>()?
    ))
}

/// hh:mm:ss convenience function
pub fn hms(seconds: i64) -> String {
    let s = seconds % 60;
    let m = (seconds / 60) % 60; 
    let h = (seconds / 3600) % 60; 
    format!("{:02}:{:02}:{:02}", h, m, s)
}

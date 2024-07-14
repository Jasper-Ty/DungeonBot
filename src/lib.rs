pub mod db;

pub mod commands;
pub mod lastmessage;
pub mod error;

use std::env;

use dotenvy::dotenv;

use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;

use db::{models, schema};
use error::{DungeonBotError, Result};

pub fn db_conn() -> SqliteConnection {
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

/// Gets the user `user_id`. Returns None if the user is not found.
pub fn get_user(conn: &mut SqliteConnection, user_id: u64) -> Result<Option<User>> {
    use schema::users::dsl::*;

    users
        .find(user_id as i64)
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(DungeonBotError::from)
}

/// Adds `points` points to user `user_id`. Returns the updated number of points.
pub fn add_points(conn: &mut SqliteConnection, user_id: u64, pts: i32) -> Result<usize> {
    use schema::users::dsl::*;
    let user_id = user_id as i64;

    diesel::update(users)
        .filter(id.eq(user_id))
        .set(points.eq(points + pts))
        .execute(conn)
        .map_err(DungeonBotError::from)
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
        env::var(key)
            .map_err(|e| 
                     DungeonBotError::EnvVarError { 
                         key: key.to_string(), 
                         source: e 
                     })?
        .parse::<u64>()
            .map_err(|e| 
                     DungeonBotError::SnowflakeParseError { 
                         snowflake: key.to_string(), 
                         source: e 
                     })?
    ))
}


pub fn env_str(key: &str) -> Result<String> {
    env::var(key)
        .map_err(|e| 
                 DungeonBotError::EnvVarError { 
                     key: key.to_string(), 
                     source: e 
                 })
}


/// hh:mm:ss convenience function
pub fn hms(seconds: i64) -> String {
    let s = seconds % 60;
    let m = (seconds / 60) % 60; 
    let h = (seconds / 3600) % 60; 
    format!("{:02}:{:02}:{:02}", h, m, s)
}

pub mod models;
pub mod schema;

mod migrations;

pub use migrations::run_migrations;

use dotenvy::dotenv;

use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;

use models::User;
use crate::env_str;
use crate::error::{DungeonBotError, Result};

/// Creates a connection to the current Dungeon database.
pub fn db_conn() -> Result<SqliteConnection> {
    dotenv().ok();

    let database_url = env_str("DATABASE_URL")?;
    SqliteConnection::establish(&database_url)
        .map_err(DungeonBotError::from)
}


/// Creates a new user with the given id. Returns None if the user doesn't exist.
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

pub fn get_points(conn: &mut SqliteConnection, user_id: u64) -> Result<Option<i32>> {
    use schema::users::dsl::*;

    users
        .find(user_id as i64)
        .select(points)
        .first(conn)
        .optional()
        .map_err(DungeonBotError::from)
}

/// Adds `pts` points to user `user_id`. Returns the updated number of points.
pub fn add_points(conn: &mut SqliteConnection, user_id: u64, pts: i32) -> Result<usize> {
    use schema::users::dsl::*;
    let user_id = user_id as i64;

    diesel::update(users)
        .filter(id.eq(user_id))
        .set(points.eq(points + pts))
        .execute(conn)
        .map_err(DungeonBotError::from)
}

/// Transfers `pts` points from user `from_id` to user `to_id`.
pub fn xfer_points(
    conn: &mut SqliteConnection,
    to_id: u64, 
    from_id: u64,
    pts: i32
) -> Result<()> {
    use schema::users::dsl::*;

    conn.transaction(|conn| {
        let to = users
            .find(to_id as i64)
            .select(User::as_select())
            .first(conn)
            .optional()?;
        if to.is_none() {
            return Err(DungeonBotError::DbUserNotFoundError(to_id))
        }

        let from = users
            .find(from_id as i64)
            .select(User::as_select())
            .first(conn)
            .optional()?;
        if from.is_none() {
            return Err(DungeonBotError::DbUserNotFoundError(from_id))
        }


        diesel::update(users)
            .filter(id.eq(to_id as i64))
            .set(points.eq(points + pts))
            .execute(conn)?;

        diesel::update(users)
            .filter(id.eq(from_id as i64))
            .set(points.eq(points - pts))
            .execute(conn)?;

        Ok(())
    })
}

/// Retrieves the [off,off + lim)-th users by aura
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

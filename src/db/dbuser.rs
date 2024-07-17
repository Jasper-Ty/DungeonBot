use diesel::prelude::*;

use crate::schema;
use crate::schema::users;

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DbUser {
    pub id: i64,
    pub points: i32
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewDbUser {
    pub id: i64,
}

use crate::error::{DungeonBotError, Result};

impl DbUser {

    /// Creates a new DbUser with id `user_id`.
    /// Returns the created or existing DbUser.
    pub fn new(conn: &mut SqliteConnection, user_id: u64) -> Result<Self> {
        use schema::users::dsl::*;
        let user_id = user_id as i64;

        let new_user = NewDbUser { id: user_id };

        diesel::insert_into(users)
            .values(&new_user)
            .on_conflict(id)
            .do_nothing()
            .returning(Self::as_returning())
            .get_result(conn)
            .map_err(DungeonBotError::from)
    }

    /// Gets the DbUser `user_id`. 
    /// Returns None if the user is not found.
    pub fn get(conn: &mut SqliteConnection, user_id: u64) -> Result<Option<Self>> {
        use schema::users::dsl::*;

        users
            .find(user_id as i64)
            .select(Self::as_select())
            .first(conn)
            .optional()
            .map_err(DungeonBotError::from)
    }

    /// Gets the DbUser `user_id`'s points.
    /// Returns None if the user is not found.
    pub fn get_points(conn: &mut SqliteConnection, user_id: u64) -> Result<Option<i32>> {
        use schema::users::dsl::*;

        users
            .find(user_id as i64)
            .select(points)
            .first(conn)
            .optional()
            .map_err(DungeonBotError::from)
    }

    /// Adds `pts` points to user `user_id`. 
    /// Returns the updated number of points.
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
                .select(Self::as_select())
                .first(conn)
                .optional()?;
            if to.is_none() {
                return Err(DungeonBotError::DbUserNotFoundError(to_id))
            }

            let from = users
                .find(from_id as i64)
                .select(Self::as_select())
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
    pub fn top(conn: &mut SqliteConnection, lim: i64, off: i64) -> Vec<Self> {
        use schema::users::dsl::*;

        users 
            .limit(lim)
            .offset(off)
            .order_by(points.desc())
            .select((id, points))
            .load(conn)
            .expect("Error loading users")
    }

}

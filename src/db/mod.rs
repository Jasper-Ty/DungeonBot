pub mod models;
pub mod schema;

mod migrations;
mod dbuser;

pub use migrations::run_migrations;
pub use dbuser::*;

use dotenvy::dotenv;

use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;

use crate::env_str;
use crate::error::{DungeonBotError, Result};

/// Creates a connection to the current Dungeon database.
pub fn db_conn() -> Result<SqliteConnection> {
    dotenv().ok();

    let database_url = env_str("DATABASE_URL")?;
    SqliteConnection::establish(&database_url)
        .map_err(DungeonBotError::from)
}

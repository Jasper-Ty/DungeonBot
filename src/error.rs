use std::{env::VarError, num::ParseIntError};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DungeonBotError {
    #[error("Database (diesel) error")]
    DbError (#[from] diesel::result::Error),

    #[error("Discord (serenity) error")]
    DiscordError (#[from] serenity::Error),

    #[error("Error retrieving environment variable `{key}`")]
    EnvVarError {
        key: String,
        #[source] 
        source: VarError
    },

    #[error("Unable to convert `{snowflake}` into snowflake")]
    SnowflakeParseError {
        snowflake: String,
        source: ParseIntError,
    },

    #[error("User {0} not found (database)")]
    DbUserNotFoundError(u64),

    #[error("User {0} not found (discord)")]
    DiscordUserNotFoundError(u64),

    #[error("Unknown")]
    Unknown,
}

pub type Result<T> = core::result::Result<T, DungeonBotError>;

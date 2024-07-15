use std::{env::VarError, error::Error, num::ParseIntError};

use thiserror::Error;

/// Big error class :flabbergasted:
#[derive(Error, Debug)]
pub enum DungeonBotError {
    #[error("Database (diesel) error")]
    DbError (#[from] diesel::result::Error),

    #[error("Error connecting to database")]
    DbConnError (#[from] diesel::ConnectionError),

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

    #[error("Global data does not have key {0}")]
    TypeMapMissingKeyError(String),

    #[error("User {0} not found (discord)")]
    DiscordUserNotFoundError(u64),

    #[error("Unknown")]
    MigrationError(Box<dyn Error + Send + Sync + 'static>),

    #[error("Other: {0}")]
    Other(String),

    #[error("Unknown")]
    Unknown,
}

pub type Result<T> = core::result::Result<T, DungeonBotError>;

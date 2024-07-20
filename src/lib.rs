pub mod db;
use db::schema;

pub mod commands;
pub mod subsystems;
pub mod error;
pub mod messagehandler;

use std::env;
use error::{DungeonBotError, Result};

use tracing::debug;

/// Convenience function for getting a type whose underlying
/// id data type is a snowflake (e.g a user id).
pub fn env_snowflake<T: From<u64>> (key: &str) -> Result<T> {
    debug!(key, "env_snowflake");
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
    debug!(key, "env_str");
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

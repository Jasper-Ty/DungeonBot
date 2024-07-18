use dotenvy::dotenv;
 
use serenity::prelude::*;
use serenity::all::{ChannelId, Message, RoleId};

use crate::db::{db_conn, DbUser};
use crate::env_snowflake;
use crate::error::{DungeonBotError, Result};
use crate::messagehandler::{MsgSubsystem, MsgSubsystemLock};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CountingError {
    #[error("Error acquiring counting lock (read)")]
    CTLockReadError,
    #[error("Error acquiring counting lock (write)")]
    CTLockWriteError,
}

#[derive(Debug, Clone)]
pub struct CountingData {
    pub num: u64,
}
impl Default for CountingData {
    fn default() -> Self {
        Self {
            num: 1000
        }
    }
}

pub struct Counting;
impl TypeMapKey for Counting {
    type Value = MsgSubsystemLock<CountingData>;
}

impl MsgSubsystem for Counting {
    type Data = CountingData;

    fn name() -> String {
        "Counting".to_string()
    }

    async fn handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        dotenv().ok();

        let ctchannel: ChannelId = env_snowflake("COUNTING_CHANNEL_ID")?;
        let ctrole: RoleId = env_snowflake("COUNTING_ROLE_ID")?;

        // Don't care if it's not in the right channel!
        if msg.channel_id != ctchannel { return Ok(()) }
        // No bots!
        if msg.author.bot { return Ok(()) }

        // Attempt to parse first word of message
        let Some(Ok(newct)) = msg.content
            .split(" ")
            .next()
            .map(str::parse::<u64>) else { return Ok(()) };

        let success = {
            let ctlock = Counting::lock(ctx).await?;
            let mut write_lock = ctlock.write()?;

            let oldct = write_lock.num;

            if newct == (oldct).rem_euclid(1000)+1 {
                (*write_lock).num = newct;
                true
            } else { false }
        };

        if success {
            let connection = &mut db_conn()?;

            if newct == 1000 {
                DbUser::add_points(connection, msg.author.id.into(), 500)?;

                /* Add 1000 role */
                let memb = msg.member(&ctx.http()).await
                    .map_err(DungeonBotError::from)?;
                memb.add_role(&ctx.http(), ctrole).await
                    .map_err(DungeonBotError::from)?;

            } else {
                DbUser::add_points(connection, msg.author.id.into(), 3)?;
            }
            set_ct(connection, newct)?;

            msg.react(&ctx.http, '✅').await
                .map_err(DungeonBotError::from)?;
        }

        Ok(())
    }
}

use diesel::SqliteConnection;
use diesel::prelude::*;
use crate::db::models::StateVar;

pub fn get_saved_ct(conn: &mut SqliteConnection) -> Result<u64> {
    use crate::db::schema::state::dsl::*;

    let new_count = StateVar { 
        key: "COUNT".to_string(),
        value: "1000".to_string()
    };

    diesel::insert_into(state)
        .values(&new_count)
        .on_conflict(key)
        .do_nothing()
        .execute(conn)
        .map_err(DungeonBotError::from)?;

    let res: String = state 
        .find("COUNT")
        .select(value)
        .first(conn)
        .map_err(DungeonBotError::from)?;

    res.parse::<u64>()
        .map_err(|_| DungeonBotError::Other("Unable to parse saved count value".to_string()))

}

pub fn set_ct(conn: &mut SqliteConnection, ct: u64) -> Result<usize> {
    use crate::db::schema::state::dsl::*;

    diesel::update(state)
        .filter(key.eq("COUNT"))
        .set(value.eq(format!("{}", ct)))
        .execute(conn)
        .map_err(DungeonBotError::from)
}

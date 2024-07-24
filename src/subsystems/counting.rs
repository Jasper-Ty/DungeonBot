use dotenvy::dotenv;
 
use serenity::{async_trait, prelude::*};
use serenity::all::{ChannelId, Message, RoleId};

use crate::db::{db_conn, DbUser};
use crate::env_snowflake;
use crate::error::{DungeonBotError, Result};

use super::subsystem::{Subsystem, SyncRwLock};

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
        let conn = &mut db_conn()
            .expect("Unable to connect to db");
        let num = Counting::get_db_ct(conn)
            .expect("No saved count in database");

        Self {
            num
        }
    }
}

type CountingLock = SyncRwLock<CountingData>;

pub struct Counting;
impl TypeMapKey for Counting {
    type Value = CountingLock;
}

impl Subsystem for Counting {
    type Data = CountingData;

    async fn message_handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        if msg.author.bot { return Ok(()) }

        dotenv().ok();

        let ctchannel: ChannelId = env_snowflake("COUNTING_CHANNEL_ID")?;
        let ctrole: RoleId = env_snowflake("COUNTING_ROLE_ID")?;

        // Don't care if it's not in the right channel!
        if msg.channel_id != ctchannel { return Ok(()) }

        // Attempt to parse first word of message
        let Some(Ok(newct)) = msg.content
            .split(" ")
            .next()
            .map(str::parse::<u64>) else { return Ok(()) };

        // Check if value is correct
        let oldct = Self::get_lock_ct(ctx).await?;
        let is_next_value = newct == (oldct).rem_euclid(1000) + 1;

        let connection = &mut db_conn()?;

        if is_next_value {
            // Set count behind lock
            Self::set_lock_ct(ctx, newct).await?;

            // Set saved count in db
            Self::set_db_ct(connection, newct)?;

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

            msg.react(&ctx.http, '✅').await
                .map_err(DungeonBotError::from)?;
        } else { 
            DbUser::add_points(connection, msg.author.id.into(), -10)?;
            msg.react(&ctx.http, '❌').await
                .map_err(DungeonBotError::from)?;
        }

        Ok(())
    }
}

use diesel::SqliteConnection;
use diesel::prelude::*;
use crate::db::models::StateVar;

impl Counting {

    pub async fn get_lock_ct(ctx: &Context) -> Result<u64> {
        let ctlock = Self::lock(ctx).await?;
        let read_lock = ctlock.read()?;
        Ok(read_lock.num)
    }

    pub async fn set_lock_ct(ctx: &Context, ct: u64) -> Result<()> {
        let ctlock = Self::lock(ctx).await?;
        let mut write_lock = ctlock.write()?;
        (*write_lock).num = ct;
        Ok(())
    }

    pub fn get_db_ct(conn: &mut SqliteConnection) -> Result<u64> {
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

    pub fn set_db_ct(conn: &mut SqliteConnection, ct: u64) -> Result<usize> {
        use crate::db::schema::state::dsl::*;

        {
            diesel::update(state)
                .filter(key.eq("COUNT"))
                .set(value.eq(format!("{}", ct)))
                .execute(conn)
                .map_err(DungeonBotError::from)
        }
    }
}

#[async_trait]
impl EventHandler for Counting {
    async fn message(&self, mut ctx: Context, msg: Message) {

        if msg.author.bot { return }

        if let Err(err) = Self::message_handler(&mut ctx, &msg).await {
            Self::error_handler(&mut ctx, &msg, err).await;
        }
    }
}

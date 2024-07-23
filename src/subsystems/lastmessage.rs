use serenity::prelude::*;
use serenity::all::{Message, UserId, RoleId, ChannelId, Member, Timestamp};

use dotenvy::dotenv;

use crate::error::DungeonBotError;
use crate::{env_snowflake, hms};
use crate::db::{db_conn, DbUser};
use crate::error::Result;

use super::subsystem::{Subsystem, AsyncRwLock};

#[derive(Debug, Clone)]
pub struct LastMessageData {
    memb: Member,
    timestamp: Timestamp,
}

impl LastMessageData {
    pub fn id(&self) -> UserId {
        self.memb.user.id
    }
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

const STREAK_MULTIPLIER: i64 = 5;
const STREAK_BONUS_MULTIPLIER: i64 = 40;

/// The underlying async data structure that holds the
/// last-message winner.
type LMLock = AsyncRwLock<Option<LastMessageData>>;

// Holds the user id of the current Last Message Winner
// Serenity uses unit structs to set up the type system for
// their global data dictionary (data member in Context)
// (Very TypeScript-y business!)
pub struct LastMessage;
impl TypeMapKey for LastMessage {
    type Value = LMLock;
}

impl Subsystem for LastMessage {
    type Data = Option<LastMessageData>;

    async fn message_handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        dotenv().ok();
        let lmchannel: ChannelId = 
            env_snowflake("LAST_MESSAGE_CHANNEL_ID")?;
        let connection = &mut db_conn()?;

        // Don't care if it's not in the right channel!
        if msg.channel_id != lmchannel { return Ok(()) }

        // Retrieve guild user
        let new = msg.member(&ctx.http).await?;

        // If winner isn't changing, no-op.
        if !Self::is_new_winner(ctx, &new).await? {
            return Ok(())
        }

        /* Two steps (2 http requests) to update last message winner:
         *
         * a. Removing role from the previous winner
         * b. Adding role to the new winner
         *
         * NOT A BIG DEAL if (a) succeeds and (2) fails:
         *      LM role now has 0 members-- meh
         * BIG DEAL if (a) fails and (2) succeeds:
         *      LM role now has >1 members-- NOT GOOD
         *
         * So we try to do (a), and if that succeeds, do (b).
         *
         * This guarantees the role will only ever have at most one member in it.
         */

        // (a)
        let lmdata = Self::pop(ctx).await?;

        // (b)
        Self::push(ctx, new.clone(), msg.timestamp).await?;

        /*
         * Then, once the Discord side is finished, the database side is much easier and much more
         * reliable, and we always try to do 
         *
         * c. Hand out points to streak holder, and bonus to streak breaker
         */

        // (c)
        if let Some(LastMessageData {
            memb: curr,
            timestamp,
        }) = lmdata {

            let dt = {
                let t0 = timestamp.timestamp();
                let t1 = msg.timestamp.timestamp();
                t1 - t0
            };

            // Award streak to previous member
            DbUser::add_points(connection, curr.user.id.into(), (dt/STREAK_MULTIPLIER) as i32)?;

            // Award streak break bonus to new member
            DbUser::add_points(connection, new.user.id.into(), (dt/STREAK_BONUS_MULTIPLIER) as i32)?;

            if dt >= 300 {
                Self::streak_message(ctx, &curr, &new, dt, lmchannel).await?;
            }
        }

        Ok(())
    }
}

impl LastMessage {
    pub async fn state(ctx: &Context) -> Result<Option<(Member, i64)>> {
        let lmlock = Self::lock(ctx).await?;
        let read_lock = lmlock.read().await?;

        Ok(read_lock.as_ref()
            .map(|LastMessageData { memb, timestamp }| 
                 (memb.clone(), Timestamp::now().timestamp() - timestamp.timestamp())
                 ))
    }

    pub async fn current_streak(ctx: &Context) -> Result<Option<i64>> {
        let lmlock = Self::lock(ctx).await?;
        let read_lock = lmlock.read().await?;

        Ok(read_lock.as_ref()
            .map(|LastMessageData { timestamp, .. }| 
                Timestamp::now().timestamp() - timestamp.timestamp()
            ))
    }

    pub async fn get_winner(ctx: &Context) -> Result<Option<Member>> {
        let lmlock = Self::lock(ctx).await?;
        let read_lock = lmlock.read().await?;

        Ok(read_lock.as_ref()
            .map(|LastMessageData { memb, .. }| memb.clone()))
    }

    pub async fn set_winner(
        ctx: &Context, 
        memb: Member, 
        timestamp: Timestamp
    ) -> Result<()> {
        let lmlock = Self::lock(ctx).await?;
        let mut write_lock = lmlock.write().await?;

        *write_lock = Some(LastMessageData { memb, timestamp });

        Ok(())
    }

    /// Checks if the current winner is the same as `new`.
    pub async fn is_new_winner(
        ctx: &Context, 
        new: &Member
    ) -> Result<bool> {
        if let Some(curr) = Self::get_winner(ctx).await? {
            Ok(curr.user.id != new.user.id)
        } else {
            Ok(true)
        }
    }

    /// Attempts to remove the current winner, and gives them
    /// the last message role
    ///
    /// This has to be done atomically, hence we write lock code in here
    async fn pop(ctx: &Context) -> Result<Option<LastMessageData>> {
        let lmrole: RoleId = 
            env_snowflake("LAST_MESSAGE_ROLE_ID")?;

        // Acquire lock
        let lmlock = Self::lock(ctx).await?;
        let mut write_lock = lmlock.write().await?;

        // Get LastMessageData
        let lmdata = write_lock.clone();

        // Remove previous winner from role
        if let Some(LastMessageData{ memb: curr, timestamp: _ }) = lmdata.as_ref() {
            curr.remove_role(&ctx.http, lmrole).await
                .map_err(DungeonBotError::from)?;
        }

        // Clear lock
        *write_lock = None;

        Ok(lmdata)
    }

    /// Sets a new winner, and gives them the last message role
    ///
    /// This has to be done atomically, hence we write lock code in here
    async fn push(
        ctx: &mut Context, 
        memb: Member,
        timestamp: Timestamp
    ) -> Result<()> {
        let lmrole: RoleId = 
            env_snowflake("LAST_MESSAGE_ROLE_ID")?;

        // Acquire lock
        let lmlock = Self::lock(ctx).await?;
        let mut write_lock = lmlock.write().await?;

        // Add new winner to role
        memb.add_role(&ctx.http, lmrole).await
            .map_err(DungeonBotError::from)?;

        // Update value in lock
        *write_lock = Some(LastMessageData {
            memb,
            timestamp
        });

        Ok(())
    }

    /// Sends a streak message
    async fn streak_message(
        ctx: &mut Context, 
        curr: &Member,
        new: &Member,
        dt: i64,
        channel: ChannelId
    ) -> Result<()> {

        let streak_message = format!(
            "😱 {} broke {}'s {} last message streak! As a bonus, they earn {} aura.", 
            new.display_name(),
            curr.display_name(),
            hms(dt),
            dt/STREAK_BONUS_MULTIPLIER
        );

        channel.say(&ctx.http, streak_message).await     
            .map_err(DungeonBotError::from)?;

        Ok(())
    }
}

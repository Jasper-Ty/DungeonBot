use serenity::prelude::*;
use serenity::all::{Message, UserId, RoleId, ChannelId, Member, Timestamp};

use dotenvy::dotenv;

use crate::error::DungeonBotError;
use crate::messagehandler::{MsgSubsystem, MsgSubsystemAsyncLock};
use crate::{env_snowflake, hms};
use crate::db::{db_conn, DbUser};
use crate::error::Result;

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
type LMLock = MsgSubsystemAsyncLock<Option<LastMessageData>>;

// Holds the user id of the current Last Message Winner
// Serenity uses unit structs to set up the type system for
// their global data dictionary (data member in Context)
// (Very TypeScript-y business!)
pub struct LastMessage;
impl TypeMapKey for LastMessage {
    type Value = LMLock;
}
impl MsgSubsystem for LastMessage {
    type Data = Option<LastMessageData>;

    fn name() -> String {
        "Last Message".to_string()
    }

    async fn handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        dotenv().ok();
        let lmchannel: ChannelId = 
            env_snowflake("LAST_MESSAGE_CHANNEL_ID")?;
        let lmrole: RoleId = 
            env_snowflake("LAST_MESSAGE_ROLE_ID")?;
        let connection = &mut db_conn()?;

        // Don't care if it's not in the right channel!
        if msg.channel_id != lmchannel { return Ok(()) }

        // Retrieve guild user
        let new = msg.member(&ctx.http).await?;

        // Get the LMLock
        let mut lmlock = LastMessage::lock(ctx).await?;

        // If winner isn't changing, no-op.
        if !is_new_winner(&lmlock, &new).await {
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
        let lmdata = pop_curr_winner(
            ctx, 
            &mut lmlock, 
            &lmrole
        ).await?;

        // (b)
        set_new_winner(
            ctx, 
            &mut lmlock, 
            &lmrole, 
            &new, 
            &msg.timestamp
        ).await?;

        /*
         * Then, once the Discord side is finished, the database side is much easier and much more
         * reliable, and we always try to do 
         *
         * c. Hand out points to streak winner
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

            // Update database value
            DbUser::add_points(connection, curr.user.id.into(), (dt/STREAK_MULTIPLIER) as i32)
                .expect("Unable to add points");

            // Update database value
            DbUser::add_points(connection, new.user.id.into(), (dt/STREAK_BONUS_MULTIPLIER) as i32)
                .expect("Unable to add points");

            if dt >= 300 {
                streak_message(ctx, &curr, &new, dt, lmchannel).await
                    .expect("Error sending streak message");
            }
        }

        Ok(())
    }
}

/// Checks if the current winner (behind `rwlock`) is the same as `new`.
async fn is_new_winner(rwlock: &LMLock, new: &Member) -> bool {
    let read_lock = rwlock.read().await;

    if let Some(LastMessageData { 
        memb: curr, 
        timestamp: _ 
    }) = read_lock.as_ref() {
        curr.user.id != new.user.id
    } else {
        true
    }
}

/// Attempts to remove the current winner in the LMLock
async fn pop_curr_winner(
    ctx: &mut Context, 
    rwlock: &mut LMLock, 
    lmrole: &RoleId
) -> Result<Option<LastMessageData>> {
    let mut write_lock = rwlock.write().await;

    // Get LastMessageData
    let lmdata = write_lock.clone();

    // Remove previous winner from role
    if let Some(LastMessageData{ memb: curr, timestamp: _ }) = lmdata.as_ref() {
        curr.remove_role(&ctx.http, lmrole).await
            .map_err(DungeonBotError::from)?;
    }

    // Clear LMLock
    *write_lock = None;

    Ok(lmdata)
}


/// Sends the streak message
async fn streak_message(
    ctx: &mut Context, 
    curr: &Member,
    new: &Member,
    dt: i64,
    lmchannel: ChannelId
) -> Result<()> {

    let streak_message = format!(
        "😱 {} broke {}'s {} last message streak! As a bonus, they earn {} aura.", 
        new.display_name(),
        curr.display_name(),
        hms(dt),
        dt/STREAK_BONUS_MULTIPLIER
    );

    lmchannel.say(&ctx.http, streak_message).await     
        .map_err(DungeonBotError::from)?;

    Ok(())
}

/// Sets a new winner in LMLock
async fn set_new_winner(
    ctx: &mut Context, 
    rwlock: &mut LMLock, 
    lmrole: &RoleId,
    new: &Member,
    timestamp: &Timestamp
) -> Result<()> {
    let mut write_lock = rwlock.write().await;

    // Add new winner to role
    new.add_role(&ctx.http, lmrole).await
        .map_err(DungeonBotError::from)?;

    // Update value in mutex
    *write_lock = Some(LastMessageData {
        memb: new.clone(),
        timestamp: timestamp.clone()
    });

    Ok(())
}

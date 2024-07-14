use std::sync::Arc;
use tokio::sync::RwLock; // Need async Mutex, will be holding across awaits
                        
use serenity::prelude::*;
use serenity::async_trait;
use serenity::all::{Message, RoleId, ChannelId, Member, Timestamp};

use dotenvy::dotenv;

use crate::error::DungeonBotError;
use crate::{env_snowflake, hms};
use crate::{add_points, new_user};
use crate::db::db_conn;
use crate::error::Result;

pub struct LastMessageHandler;

#[derive(Clone)]
struct LastMessageData {
    memb: Member,
    timestamp: Timestamp,
}

/// The underlying async data structure that holds the
/// last-message winner.
type LMLock = Arc<RwLock<Option<LastMessageData>>>;

// Holds the user id of the current Last Message Winner
// Serenity uses unit structs to set up the type system for
// their global data dictionary (data member in Context)
// (Very TypeScript-y business!)
struct LastMessage;
impl TypeMapKey for LastMessage {
    type Value = LMLock;
}

/// Get the LastMessage RwLock from Context
async fn get_lmlock(ctx: &mut Context) -> LMLock {
    ctx.data.read().await.get::<LastMessage>()
        .expect("Expected LastMessageWinner in TypeMap.")
        .clone()
}

pub async fn install_lastmessage_key(client: &mut Client) {
    let mut data = client.data.write().await;
    data.insert::<LastMessage>(Arc::new(RwLock::new(None)));
}

// The message handler that processes every message in #last-message,
// and updates roles accordingly.
#[async_trait]
impl EventHandler for LastMessageHandler {
    async fn message(&self, mut ctx: Context, msg: Message) {

        dotenv().ok();
        let lmchannel: ChannelId = 
            env_snowflake("LAST_MESSAGE_CHANNEL_ID")
            .expect("Unable to get Last Message Channel Id");
        let lmrole: RoleId = 
            env_snowflake("LAST_MESSAGE_ROLE_ID")
            .expect("Unable to get Last Message Role Id");

        let connection = &mut db_conn()
            .expect("Unable to connect to database");

        // Don't care if it's not in the right channel!
        if msg.channel_id != lmchannel { return }
        // No bots!
        if msg.author.bot { return }

        // Create a new database entry and retrieve guild user
        new_user(connection, msg.author.id.into());
        let new = msg.member(&ctx.http).await 
            .expect("Unable to find guild member");

        // Get the LMLock
        let mut rwlock = get_lmlock(&mut ctx).await;

        // If winner isn't changing, no-op.
        if !is_new_winner(&rwlock, &new).await {
            return
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
            &mut ctx, 
            &mut rwlock, 
            &lmrole
        ).await
            .expect("Error popping current winner");

        // (b)
        set_new_winner(
            &mut ctx, 
            &mut rwlock, 
            &lmrole, 
            &new, 
            &msg.timestamp
        ).await
            .expect("Error setting new winner");

        // Now we hand out points
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
            add_points(connection, curr.user.id.into(), (dt/5) as i32)
                .expect("Unable to add points");

            // Update database value
            add_points(connection, new.user.id.into(), (dt/40) as i32)
                .expect("Unable to add points");

            if dt >= 300 {
                streak_message(&mut ctx, &curr, &new, dt, lmchannel).await
                    .expect("Error sending streak message");
            }
        } 
    }
}

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
        dt/100
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

use std::sync::Arc;
use tokio::sync::Mutex; // Need async Mutex, will be holding across awaits
                        
use serenity::prelude::*;
use serenity::async_trait;
use serenity::all::{Message, RoleId, ChannelId, Member, Timestamp};

use dotenvy::dotenv;

use crate::add_points;
use crate::env_snowflake;
use crate::hms;
use crate::{new_user, establish_connection};

pub struct LastMessageHandler;

struct LastMessageData {
    memb: Member,
    timestamp: Timestamp,
}

// Holds the user id of the current Last Message Winner
// Serenity uses unit structs to set up the type system for
// their global data dictionary (data member in Context)
// (Very TypeScript-y business!)
struct LastMessage;
impl TypeMapKey for LastMessage {
    type Value = Arc<Mutex<Option<LastMessageData>>>;
}

pub async fn install_lastmessage_key(client: &mut Client) {
    let mut data = client.data.write().await;
    data.insert::<LastMessage>(Arc::new(Mutex::new(None)));
}

// The message handler that processes every message in #last-message,
// and updates roles accordingly.
#[async_trait]
impl EventHandler for LastMessageHandler {
    async fn message(&self, ctx: Context, msg: Message) {

        dotenv().ok();
        let last_message_channel_id: ChannelId = 
            env_snowflake("LAST_MESSAGE_CHANNEL_ID")
            .expect("Unable to get Last Message Channel Id");
        let last_message_role_id: RoleId = 
            env_snowflake("LAST_MESSAGE_ROLE_ID")
            .expect("Unable to get Last Message Role Id");

        let connection = &mut establish_connection();

        // Don't care if it's not in the right channel!
        if msg.channel_id != last_message_channel_id { return }
        // No bots!
        if msg.author.bot { return }

        // Insert into database
        new_user(connection, msg.author.id.into());

        // Acquire mutex
        let data_read = ctx.data.read().await;
        let mtx = data_read.get::<LastMessage>()
            .expect("Expected LastMessageWinner in TypeMap.")
            .clone();
        let mut lmdata = mtx.lock().await;

        // Retrieve the guild member of the message author
        let Ok(new) = msg.member(&ctx.http).await else { return };

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
        if let Some(LastMessageData {
            memb: curr,
            timestamp,
        }) = lmdata.as_ref() {

            let dt = {
                let t0 = timestamp.timestamp();
                let t1 = msg.timestamp.timestamp();
                t1 - t0
            };
            let curr_name = curr.display_name().to_string();
            let new_name = new.display_name().to_string();

            // No-op if winner hasn't changed
            if curr.user.id == new.user.id {
                return 
            }

            // Remove previous winner from role
            if let Err(e) = curr.remove_role(&ctx.http, last_message_role_id).await {
                println!("Error removing role: {:?}", e);
                return
            }

            // Update database value
            add_points(connection, curr.user.id.into(), (dt/5) as i32)
                .expect("Unable to add points");

            // Update value in mutex
            *lmdata = None;

            if dt >= 300 {
                if let Err(e) = msg.channel_id.say(
                    &ctx.http, 
                    format!("😱 {} broke {}'s {} last message streak!", new_name, curr_name, hms(dt))
                ).await {
                    println!("Error sending message {:?}", e);
                }
            }
        } 

        // (b)
        {
            // Add new winner to role
            if let Err(e) = new.add_role(&ctx.http, last_message_role_id).await {
                println!("Error adding role: {:?}", e);
                return
            }

            // Update value in mutex
            *lmdata = Some(LastMessageData {
                memb: new,
                timestamp: msg.timestamp
            });
        }
    }
}

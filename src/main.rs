use std::sync::Arc;
use std::fs;
use tokio::sync::Mutex; // Need async Mutex, will be holding across awaits

use toml::Table;
use serenity::all::{ChannelId, Member, Message, RoleId, Timestamp};
use serenity::async_trait;
use serenity::prelude::*;

// I really *shouldn't* have members in this struct at all,
// but it's a convenient solution for now
struct LastMessageHandler {
    last_message_channel_id: ChannelId,
    last_message_role_id: RoleId,
}


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

// The message handler that processes every message in #last-message,
// and updates roles accordingly.
#[async_trait]
impl EventHandler for LastMessageHandler {
    async fn message(&self, ctx: Context, msg: Message) {

        // Don't care if it's not in the right channel!
        if msg.channel_id != self.last_message_channel_id { return }
        // No bots!
        if msg.author.bot { return }

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
            let name = curr.display_name().to_string();

            // No-op if winner hasn't changed
            if curr.user.id == new.user.id {
                return 
            }

            // Remove previous winner from role
            if let Err(e) = curr.remove_role(&ctx.http, self.last_message_role_id).await {
                println!("Error removing role: {:?}", e);
                return
            }

            // Update value in mutex
            *lmdata = None;

            if dt >= 600 {
                if let Err(e) = msg.channel_id.say(
                    &ctx.http, 
                    format!("😱! {} held the last message for {} seconds", name, dt)
                ).await {
                    println!("Error sending message {:?}", e);
                }
            }
        } 

        // (b)
        {
            // Add new winner to role
            if let Err(e) = new.add_role(&ctx.http, self.last_message_role_id).await {
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

#[tokio::main]
async fn main() {
    // I hate .env files, let's do something cool like TOML
    let secrets = fs::read_to_string("secrets.toml")
        .as_deref()
        .map(str::parse::<Table>)
        .unwrap()
        .unwrap();

    let bot_token = secrets["bot_token"].as_str().unwrap();
    let last_message_channel_id = secrets["last_message_channel"].as_integer().unwrap() as u64;
    let last_message_role_id = secrets["last_message_role"].as_integer().unwrap() as u64;

    let intents = GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    let handler = LastMessageHandler {
        last_message_channel_id: ChannelId::new(last_message_channel_id),
        last_message_role_id: RoleId::new(last_message_role_id),
    };

    let mut client = 
        Client::builder(&bot_token, intents)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    // Add LastMessageWinner to the global data dictionary
    {
        let mut data = client.data.write().await;
        data.insert::<LastMessage>(Arc::new(Mutex::new(None)));
    }

    // Let's go!
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

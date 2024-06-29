use std::sync::Arc;
use tokio::sync::Mutex; // Need async Mutex, will be holding across awaits
use std::fs;

use toml::Table;
use serenity::all::{ChannelId, Member, Message, RoleId};
use serenity::async_trait;
use serenity::prelude::*;

// I really *shouldn't* have members in this struct at all,
// but it's a convenient solution for now
struct Handler {
    last_message_channel_id: ChannelId,
    last_message_role_id: RoleId,
}

// Holds the user id of the current Last Message Winner
// Serenity uses unit structs to set up the type system for
// their global data dictionary (data member in Context)
// (Very TypeScript-y business!)
struct LastMessageWinner;
impl TypeMapKey for LastMessageWinner {
    type Value = Arc<Mutex<Option<Member>>>;
}

// The message handler that processes every message in #last-message,
// and updates roles accordingly.
#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {

        // Don't care if it's not in the right channel!
        if msg.channel_id != self.last_message_channel_id { return }

        // Acquire mutex
        let data_read = ctx.data.read().await;
        let mtx = data_read.get::<LastMessageWinner>()
            .expect("Expected LastMessageWinner in TypeMap.")
            .clone();
        let mut mtx_lock = mtx.lock().await;

        // Retrieve the guild member of the message author
        let Ok(new_winner) = msg.member(&ctx.http).await else { return };

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
        if let Some(curr_winner) = &mtx_lock.as_ref() {
            // No-op if winner hasn't changed
            if curr_winner.user.id == new_winner.user.id {
                return 
            }

            // Remove previous winner from role
            if let Err(e) = curr_winner.remove_role(&ctx.http, self.last_message_role_id).await {
                println!("Error removing role: {:?}", e);
                return
            }
            // Update value in mutex
            *mtx_lock = None;
        }

        // (b)
        {
            // Add new winner to role
            if let Err(e) = new_winner.add_role(&ctx.http, self.last_message_role_id).await {
                println!("Error adding role: {:?}", e);
                return
            }

            // Update value in mutex
            *mtx_lock = Some(new_winner);
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

    let handler = Handler {
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
        data.insert::<LastMessageWinner>(Arc::new(Mutex::new(None)));
    }

    // Let's go!
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

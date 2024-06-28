use std::fs;

use toml::Table;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "pong!").await {
                println!("Error sending message: {why:?}");
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let secrets = fs::read_to_string("secrets.toml")
        .as_deref()
        .map(str::parse::<Table>)
        .unwrap()
        .unwrap();

    let bot_token = secrets["bot_token"].as_str().unwrap();

    let intents = GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = 
        Client::builder(&bot_token, intents).event_handler(Handler).await.unwrap();

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

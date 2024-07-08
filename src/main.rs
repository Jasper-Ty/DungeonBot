use std::env;
use ::serenity::all::GuildId;

use dotenvy::dotenv;

use poise::serenity_prelude as serenity;
use serenity::prelude::*;

use dungeonbot::lastmessage::{install_lastmessage_key, LastMessageHandler};
use dungeonbot::commands::{ping, register, leaderboard};

#[tokio::main]
async fn main() {

    dotenv().ok();

    let guild_id = { 
        let s = env::var("GUILD_ID")
            .expect("GUILD_ID must be set");
        let u = s.parse::<u64>()
            .expect("GUILD_ID is not an integer");
        GuildId::new(u)
    };

    let bot_token = env::var("BOT_TOKEN")
        .expect("BOT_TOKEN must be set");

    let intents = GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    let handler = LastMessageHandler;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), register(), leaderboard()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await?;
                Ok (dungeonbot::commands::Data)
            })
        })
        .build();

    let mut client = Client::builder(&bot_token, intents)
        .framework(framework)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    // Add LastMessageWinner to the global data dictionary
    install_lastmessage_key(&mut client).await;

    // Let's go!
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

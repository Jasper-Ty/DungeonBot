use std::env;

use dotenvy::dotenv;

use serenity::prelude::*;
use serenity::all::GuildId;

use dungeonbot::lastmessage::{install_lastmessage_key, LastMessageHandler};
use dungeonbot::commands::{aura, leaderboard, ping, register};
use dungeonbot::error::{DungeonBotError, Result};

#[tokio::main]
async fn main() -> Result<()> {

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


    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ping(), register(), leaderboard(), aura()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await?;
                Ok (dungeonbot::commands::Data)
            })
        })
        .build();
    
    // Build client
    let mut client = Client::builder(&bot_token, intents)
        .framework(framework)
        .event_handler(LastMessageHandler)
        .await?; 

    // Add LastMessageWinner to the global data dictionary
    install_lastmessage_key(&mut client).await;

    // Let's go!
    client.start()
        .await
        .map_err(DungeonBotError::from)
}

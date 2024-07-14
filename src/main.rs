use dotenvy::dotenv;

use dungeonbot::db::run_migrations;
use dungeonbot::{env_snowflake, env_str, db_conn};
use serenity::prelude::*;
use serenity::all::GuildId;

use dungeonbot::lastmessage::{install_lastmessage_key, LastMessageHandler};
use dungeonbot::commands::{aura, leaderboard, ping, register};
use dungeonbot::error::{DungeonBotError, Result};

#[tokio::main]
async fn main() -> Result<()> {

    dotenv().ok();

    // run pending migrations
    run_migrations(&mut db_conn())?;

    let guild_id: GuildId = env_snowflake("GUILD_ID")?;
    let bot_token = env_str("BOT_TOKEN")?;

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

use dotenvy::dotenv;

use dungeonbot::db::{db_conn, run_migrations};
use dungeonbot::messagehandler::{MessageHandler, MsgSubsystem};
use dungeonbot::{env_snowflake, env_str};
use serenity::prelude::*;
use serenity::all::GuildId;

use dungeonbot::subsystems::{LastMessage, Counting};
use dungeonbot::commands::dungeonbot_framework;
use dungeonbot::error::{DungeonBotError, Result};

#[tokio::main]
async fn main() -> Result<()> {

    dotenv().ok();

    // Run pending migrations
    {
        let conn = &mut db_conn()?;
        run_migrations(conn)?;
    }

    let guild_id: GuildId = env_snowflake("GUILD_ID")?;
    let bot_token = env_str("BOT_TOKEN")?;

    let intents = GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    // Build framework
    let framework = dungeonbot_framework(guild_id);

    // Build client
    let mut client = Client::builder(&bot_token, intents)
        .framework(framework)
        .event_handler(MessageHandler)
        .await?; 

    LastMessage::install_data(&mut client).await;
    Counting::install_data(&mut client).await;

    // Let's go!
    client.start()
        .await
        .map_err(DungeonBotError::from)
}

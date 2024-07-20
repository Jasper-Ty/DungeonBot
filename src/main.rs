use tracing::info;
use tracing_subscriber;

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

    tracing_subscriber::fmt::init();

    info!("Running pending migrations");
    {
        let conn = &mut db_conn()?;
        run_migrations(conn)?;
    }
    info!("Done");

    let guild_id: GuildId = env_snowflake("GUILD_ID")?;
    let bot_token = env_str("BOT_TOKEN")?;
    let intents = GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::DIRECT_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    info!("Building poise framework");
    let framework = dungeonbot_framework(guild_id);
    info!("Done");

    info!("Building serenity client");
    let mut client = Client::builder(&bot_token, intents)
        .framework(framework)
        .event_handler(MessageHandler)
        .await?; 
    info!("Done");

    info!("Installing subsystem data");
    LastMessage::install_data(&mut client).await;
    Counting::install_data(&mut client).await;
    info!("Done");

    info!("Now starting DungeonBot!");
    client.start()
        .await
        .map_err(DungeonBotError::from)
}

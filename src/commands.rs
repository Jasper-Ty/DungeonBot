use std::fmt::Write;
use std::error::Error;

use serenity::all::{Member, UserId};

use crate::models::User;
use crate::db::{get_user, new_user, top_users, xfer_points};
use crate::error::{DungeonBotError, Result};
use crate::db::db_conn;

#[derive(Debug)]
pub struct Data;
type Context<'a> = poise::Context<'a, Data, DungeonBotError>;

#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<()> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Displays the leaderboard of the users with the
/// highest aura in the server
#[poise::command(
    slash_command,
    prefix_command
)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Page number"] page: Option<u64>
) -> Result<()> {
    let mut output: String = String::new();

    let pagenum = page.unwrap_or(1) as i64;
    let pagestr = format!("page {}", pagenum);

    output.push_str(&format!("Leaderboard {:>33}\n", pagestr));
    output.push_str("=============================================\n");
    output.push_str("Rank Username                            Aura\n");

    let connection = &mut db_conn()?;

    let offset = (pagenum-1) * 10;
    let mut i = offset + 1;
    for user in top_users(connection, 10, offset) {
        let User {
            id: user_id,
            points: pts
        } = user;
        let user_id = user_id as u64;

        let user = UserId::new(user_id)
            .to_user(&ctx.http())
            .await
            .expect("Unable to find user");

        let entry = format!("{:>4} {:<30}{:>10}\n", i, user.name, pts);
        output.push_str(&entry);
        i += 1;
    }

    ctx.say(format!("```\n{}\n```", output)).await?;

    Ok(())
}

/// Displays your aura
#[poise::command(
    slash_command,
    prefix_command,
    subcommands("aura_give")
)]
pub async fn aura(ctx: Context<'_>) -> Result<()> {
    let user_id: u64 = ctx.author().id.into();
    let connection = &mut db_conn()?;

    // Retrieve points from db
    let User {
        id,
        points
    } = get_user(connection, user_id)?
        .ok_or(DungeonBotError::DbUserNotFoundError(user_id))?;

    // Need to do this to get username
    let user = UserId::new(id as u64)
        .to_user(&ctx.http())
        .await
        .map_err(DungeonBotError::from)?;

    ctx.say(format!("{}, you have {} aura.", user.name, points)).await?;

    Ok(())
}

/// Donates aura to someone
#[poise::command(
    slash_command,
    rename="give",
    on_error="error_handler",
)]
pub async fn aura_give(
    ctx: Context<'_>,
    #[description="Recipient"] to: Member,
    #[description="Amount of aura to give"] pts: u32,
) -> Result<()> {
    let to_id: u64 = to.user.id.into();
    let from_id: u64 = ctx.author().id.into();


    let connection = &mut db_conn()?;
    new_user(connection, to_id);
    new_user(connection, from_id);
    xfer_points(connection, to_id, from_id, pts as i32)?; 

    let from = ctx.author_member().await
        .ok_or(DungeonBotError::DiscordUserNotFoundError(from_id))?;

    let reply = format!(
        "Transferred {} aura from {} to {}.", 
        pts, 
        from.display_name(), 
        to.display_name());
    ctx.say(reply).await?;
    Ok(())
}

#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

async fn error_handler(error: poise::FrameworkError<'_, Data, DungeonBotError>) {
    if let Some(ctx) = error.ctx() {
        let mut err_reply_msg = format!("Oh noes, an error <:flabbergasted:1250998996596555817>. Please let Jasper know about this immediately.\n");
        match error.source() {
            Some(source_err) => write!(
                err_reply_msg, "```{:?}\n{}```", source_err, source_err).unwrap(),
            None => write!(err_reply_msg, "No source error?").unwrap(),
        }
        ctx.say(err_reply_msg).await
            .expect("Unable to send error message");
    }
}

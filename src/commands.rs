use serenity::all::UserId;

use crate::models::User;
use crate::{db_conn, get_user, top_users};
use crate::error::{DungeonBotError, Result};

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
    prefix_command)
]
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

    let connection = &mut db_conn();

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
    prefix_command)
]
pub async fn aura(ctx: Context<'_>) -> Result<()> {
    let author = ctx.author();
    let user_id: u64 = author.id.into();
    let connection = &mut db_conn();

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

#[poise::command(prefix_command)]
pub async fn register(ctx: Context<'_>) -> Result<()> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

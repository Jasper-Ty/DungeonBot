use std::collections::HashSet;
use std::fmt::Write;

use poise::{CreateReply, FrameworkError};
use serenity::all::{parse_message_url, Member, Timestamp, UserId};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::subsystems::{Counting, LastMessage};
use crate::env_snowflake;
use crate::db::{db_conn, DbUser};
use crate::error::{DungeonBotError, Result};

#[derive(Debug)]
pub struct Data;
type Context<'a> = poise::Context<'a, Data, DungeonBotError>;

/// Displays the leaderboard of the users with the
/// highest aura in the server
#[poise::command(
    slash_command,
    guild_only,
)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Page number"] 
    #[min=1]
    #[max=10000]
    page: Option<i64>
) -> Result<()> {
    let page = page.unwrap_or(1);

    let lmstate = LastMessage::state(
        ctx.serenity_context()
    ).await?;

    let connection = &mut db_conn()?;

    let mut fields = vec![];

    let offset = (page-1) * 10;
    let mut i = offset + 1;
    for user in DbUser::top(connection, 10, offset) {
        let DbUser {
            id: user_id,
            points: pts
        } = user;
        let user_id = user_id as u64;

        let user = UserId::new(user_id)
            .to_user(&ctx.http())
            .await
            .expect("Unable to find user");

        let mut title = format!("{}. {}", i, user.name.to_string());
        let mut body = format!("{} aura", pts);

        if let Some((winner, streak)) = &lmstate {
            if winner.user.id == user.id {
                title.push_str(" ⭐");

                let streak_pts = streak/5;
                body = format!("{} ({} total + {} current streak)", 
                            pts as i64 + streak_pts,
                            pts, 
                            streak_pts
                            )
            }
        }

        fields.push((title, body, false));
        i += 1;
    }

    let npages = DbUser::count(connection)?/10;
    let footer = CreateEmbedFooter::new(format!("Page {}/{}", page, npages));
    let embed = CreateEmbed::new()
        .title("The Friendship Dungeon Aura Leaderboard")
        .fields(fields)
        .footer(footer)
        .timestamp(Timestamp::now());

    let builder = CreateReply::default()
        .embed(embed);

    ctx.send(builder).await?;

    Ok(())
}


#[poise::command(
    slash_command,
    guild_only,
    subcommands("aura_show", "aura_give", "aura_add")
)]
pub async fn aura(_: Context<'_>) -> Result<()> { Ok(()) }

/// Displays your aura.
#[poise::command(
    slash_command,
    rename="show",
    on_error="error_handler",
)]
async fn aura_show(ctx: Context<'_>) -> Result<()> {
    let user_id: u64 = ctx.author().id.into();
    let connection = &mut db_conn()?;

    let lmstate = LastMessage::state(ctx.serenity_context()).await?;

    // Retrieve points from db
    let DbUser {
        id:_,
        points
    } = DbUser::get(connection, user_id)?
        .ok_or(DungeonBotError::DbUserNotFoundError(user_id))?;

    let name = ctx.author_member().await
        .ok_or(DungeonBotError::DbUserNotFoundError(user_id))?
        .display_name()
        .to_string();


    let reply = match lmstate {
        Some((winner, streak)) if winner.user.id == user_id 
        => format!("{}, you have {} aura. ({} total + {} current streak)", 
                   name, 
                   points as i64 + streak/5, 
                   points, 
                   streak/5),
        _ => format!("{}, you have {} aura.", name, points),
    };
    ctx.say(reply).await?;

    Ok(())
}

/// Donates aura to someone
#[poise::command(
    slash_command,
    guild_only,
    rename="give",
    on_error="error_handler",
)]
async fn aura_give(
    ctx: Context<'_>,
    #[description="Recipient"] to: Member,
    #[description="Amount of aura to give"] 
    #[min=1]
    pts: i32,
) -> Result<()> {
    let to_id: u64 = to.user.id.into();
    let from_id: u64 = ctx.author().id.into();

    if to.user.bot {
        ctx.say("No.").await?;
        return Ok(())
    }

    let connection = &mut db_conn()?;
    let to_db = DbUser::new(connection, to_id)?;
    let from_db = DbUser::new(connection, from_id)?;

    if from_db.points < 0 {
        ctx.say("You are in aura debt.").await?;
        return Ok(())
    }

    if from_db.points < pts {
        let reply = format!(
            "Not enough points to complete this transaction!\nYou have {}, but you are trying to give {}",
            from_db.points, 
            pts
            );
        ctx.say(reply).await?;
        return Ok(())
    }

    // Overflow check
    if let None = to_db.points.checked_add(pts) {
        let reply = format!("Sorry, this would cause an integer overflow lol.");
        ctx.say(reply).await?;
        return Ok(())
    }

    DbUser::xfer_points(connection, to_id, from_id, pts)?; 

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

/// [JASPER ONLY] Adds aura to a member
#[poise::command(
    slash_command,
    owners_only,
    guild_only,
    rename="add",
    on_error="error_handler",
)]
async fn aura_add(
    ctx: Context<'_>,
    #[description="Recipient"] to: Member,
    #[description="Amount of aura to give"] 
    pts: i32,
) -> Result<()> {
    let to_id: u64 = to.user.id.into();

    let connection = &mut db_conn()?;
    let to_db = DbUser::new(connection, to_id)?;

    // Overflow check
    if let None = to_db.points.checked_add(pts) {
        let reply = format!("Sorry, this would cause an integer overflow lol.");
        ctx.say(reply).await?;
        return Ok(())
    }

    DbUser::add_points(connection, to_id, pts)?; 

    let reply = format!(
        "Added {} aura to {}.", 
        pts, 
        to.display_name());
    ctx.say(reply).await?;
    Ok(())
}


#[poise::command(
    slash_command,
    guild_only,
    subcommands("count_show", "count_set")
)]
pub async fn count(_: Context<'_>) -> Result<()> { Ok(()) }

/// Displays the current count
#[poise::command(
    slash_command,
    guild_only,
    rename="show",
    on_error="error_handler",
)]
async fn count_show(ctx: Context<'_>) -> Result<()> {
    let ct = Counting::get_lock_ct(ctx.serenity_context()).await?;

    let reply = format!("The current count is {}", ct);
    ctx.say(reply).await?;

    Ok(())
}

/// Sets the current count (JASPER ONLY)
#[poise::command(
    slash_command,
    owners_only,
    guild_only,
    rename="set",
    on_error="error_handler",
)]
async fn count_set(
    ctx: Context<'_>,
    #[description="Number to set"] 
    #[max=1000]
    #[min=1]
    count: u64,
) -> Result<()> {

    let conn = &mut db_conn()?;
    Counting::set_lock_ct(ctx.serenity_context(), count).await?;
    Counting::set_db_ct(conn, count)?;

    let reply = format!("Successfully set count to {}", count);
    ctx.say(reply).await?;

    Ok(())
}

/// Pins a message (500 aura)
#[poise::command(
    slash_command,
    guild_only,
    on_error="error_handler",
)]
async fn pin(
    ctx: Context<'_>,
    #[description="Link to message"] 
    msg: String,
) -> Result<()> {
    let user_id: u64 = ctx.author().id.into();

    let conn = &mut db_conn()?;
    let pts = DbUser::get_points(conn, user_id)?.unwrap();

    if pts >= 500 {
        if let Some((_, channel_id, message_id)) = parse_message_url(&msg) {
            // TODO: Bug: check against guild_id, channel_id
            let message = ctx.http().get_message(channel_id, message_id).await?;
            message.pin(ctx.http()).await?;
            ctx.reply("Success.").await?;
            DbUser::add_points(conn, user_id, -500)?;
        } else {
            ctx.reply("Failure.").await?;
        }
    } else {
        ctx.reply("Insufficient aura.").await?;
    }

    Ok(())
}

/// Unpins a message (1000 aura)
#[poise::command(
    slash_command,
    guild_only,
    on_error="error_handler",
)]
async fn unpin(
    ctx: Context<'_>,
    #[description="Link to message"] 
    msg: String,
) -> Result<()> {
    let user_id: u64 = ctx.author().id.into();

    let conn = &mut db_conn()?;
    let pts = DbUser::get_points(conn, user_id)?.unwrap();

    if pts >= 1000 {
        if let Some((_, channel_id, message_id)) = parse_message_url(&msg) {
            // TODO: Bug: check against guild_id
            let message = ctx.http().get_message(channel_id, message_id).await?;
            message.unpin(ctx.http()).await?;
            ctx.reply("Success.").await?;
            DbUser::add_points(conn, user_id, -1000)?;
        } else {
            ctx.reply("Failure.").await?;
        }
    } else {
        ctx.reply("Insufficient aura.").await?;
    }

    Ok(())
}

/// Displays this help message
#[poise::command(
    slash_command,
    guild_only,
    on_error="error_handler",
)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "Meow!",
        show_subcommands: true,
        include_description: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

async fn error_handler(framework_error: poise::FrameworkError<'_, Data, DungeonBotError>) {
    let header = 
        "Oh noes, an error \
        <:flabbergasted:1250998996596555817>. \
        Please let Jasper know about this immediately.\n";

    match framework_error {
        FrameworkError::NotAnOwner { ctx, .. } => {
            let reply = "Sorry, only Jasper is allowed to use this command";
            ctx.say(reply).await
                .expect("Unable to send error handler reply");
        }
        FrameworkError::Command { ref error, ctx, .. } => {
            let mut reply = header.to_string();

            reply.push_str("```\n");
            reply.push_str("[Command Error]\n");
            write!(reply, "{}\n", error).unwrap();
            reply.push_str("```");

            ctx.say(reply).await
                .expect("Unable to send error handler reply");
        }
        FrameworkError::CommandPanic { payload, ctx, .. } => {
            let mut reply = header.to_string();

            reply.push_str("```\n");
            reply.push_str("[Command Panic]\n");
            write!(reply, "{:?}\n", payload).unwrap();
            reply.push_str("```");

            ctx.say(reply).await
                .expect("Unable to send error handler reply");
        }
        _ => {
        }
    }
}

use serenity::all::GuildId;

/// Wrapper for the framework building
pub fn dungeonbot_framework(guild_id: GuildId) -> poise::Framework<Data, DungeonBotError>{

    let jasper_id: UserId = env_snowflake("JASPER_ID")
        .expect("JASPER_ID should be in environment");

    let mut owners = HashSet::new();
    owners.insert(jasper_id);

    let options = poise::FrameworkOptions {
        commands: vec![leaderboard(), aura(), count(), pin(), unpin(), help()],
        owners,
        ..Default::default()
    };

    poise::Framework::builder()
        .options(options)
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id).await?;
                Ok(Data)
            })
        })
        .build()
}


use std::collections::HashSet;
use std::fmt::Write;

use poise::{CreateReply, FrameworkError};
use serenity::all::{Member, Timestamp, UserId};
use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use crate::counting::{set_ct, Counting};
use crate::env_snowflake;
use crate::lastmessage::LastMessage;
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

    let lmdata = {
        let lmlock = ctx.serenity_context().data.read().await.get::<LastMessage>()
            .ok_or(DungeonBotError::TypeMapMissingKeyError("LastMessage".to_string()))?
            .clone();

        let read_lock = lmlock.read().await;

        read_lock.clone()         
    };

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

        let field_title = match lmdata {
            Some(ref lmdata) if lmdata.id() == user.id => {
                format!("{}. {} ⭐", i, user.name)
            },
            _ => format!("{}. {}", i, user.name.to_string())
        };

        let field_body = match lmdata {
            Some(ref lmdata) if lmdata.id() == user.id => {
                let dt = ctx.created_at().timestamp() - lmdata.timestamp().timestamp();
                format!("{} aura ({} total + {} current streak)", pts as i64 + dt/5, pts, dt/5)
            },
            _ => format!("{} aura", pts),
        };

        fields.push((field_title, field_body, false));
        i += 1;
    }

    let footer = CreateEmbedFooter::new(format!("Page {}", page));
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
    subcommands("aura_show", "aura_give")
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

    let streak = {
        let lmlock = ctx.serenity_context().data.read().await.get::<LastMessage>()
            .ok_or(DungeonBotError::TypeMapMissingKeyError("LastMessage".to_string()))?
            .clone();

        let read_lock = lmlock.read().await;

        match read_lock.as_ref() {
            Some(lmdata) if lmdata.id() == user_id => {
                let t0 = lmdata.timestamp().timestamp();
                let t1 = ctx.created_at().timestamp();
                Some(t1-t0)
            },
            _ => None
        }
    };

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
        
    let reply = match streak {
        Some(t) => format!("{}, you have {} aura. ({} total + {} current streak)", name, points as i64 + t/5, points, t/5),
        None => format!("{}, you have {} aura.", name, points),
    };
    ctx.say(reply).await?;

    Ok(())
}

/// Donates aura to someone
#[poise::command(
    slash_command,
    owners_only,
    guild_only,
    rename="give",
    on_error="error_handler",
)]
async fn aura_give(
    ctx: Context<'_>,
    #[description="Recipient"] to: Member,
    #[description="Amount of aura to give"] pts: u32,
) -> Result<()> {
    let to_id: u64 = to.user.id.into();
    let from_id: u64 = ctx.author().id.into();

    let connection = &mut db_conn()?;
    DbUser::new(connection, to_id)?;
    DbUser::new(connection, from_id)?;
    DbUser::xfer_points(connection, to_id, from_id, pts as i32)?; 

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
    /* Get current count */
    let ct = {
        let ctx = ctx.serenity_context();
        let ctlock = Counting::acquire_lock(ctx).await?;
        let read_lock = ctlock.read()?;

        read_lock.num
    };

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
    /* Set current count */
    {
        let ctx = ctx.serenity_context();
        let ctlock = Counting::acquire_lock(ctx).await?;
        let mut write_lock = ctlock.write()?;

        write_lock.num = count
    };

    /* Update database value */
    let conn = &mut db_conn()?;
    set_ct(conn, count)?;

    let reply = format!("Successfully set count to {}", count);
    ctx.say(reply).await?;

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
        commands: vec![leaderboard(), aura(), count()],
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

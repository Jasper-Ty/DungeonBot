use std::fmt::Write;

use serenity::all::Message;
use serenity::prelude::*;

use crate::db::{db_conn, DbUser};
use crate::subsystems::{Counting, LastMessage, Tax, Subsystem};
use crate::error::DungeonBotError;

use serenity::async_trait;

/// This unit struct denominates all message handlers
/// via Serenity's EventHandler trait
pub struct MessageHandler;

#[async_trait]
impl EventHandler for MessageHandler {
    async fn message(&self, mut ctx: Context, msg: Message) {

        // No bots!
        if msg.author.bot { return }

        {
            let connection = &mut db_conn().unwrap();
            DbUser::new(connection, msg.author.id.into()).unwrap();
        }

        let res = Ok(())
            .and(LastMessage::message_handler(&mut ctx, &msg).await)
            .and(Counting::message_handler(&mut ctx, &msg).await)
            .and(Tax::message_handler(&mut ctx, &msg).await);

        match res {
            Err(e) => error_handler(&mut ctx, &msg, e).await,
            _ => {}
        }
    }
}

async fn error_handler(ctx: &mut Context, msg: &Message, err: DungeonBotError) {
    let header = 
        "Oh noes, an error \
        <:flabbergasted:1250998996596555817>. \
        Please let Jasper know about this immediately.\n";

    let mut reply = header.to_string();
    if let Ok(channel_id) = msg.channel(&ctx.http).await {
        reply.push_str("```\n");
        reply.push_str("[Message Subsystem Error]\n");
        write!(reply, "{}\n", err).unwrap();
        reply.push_str("```");
        channel_id.id().say(&ctx.http, reply).await
            .expect("Unable to send error handler reply");
    }
}

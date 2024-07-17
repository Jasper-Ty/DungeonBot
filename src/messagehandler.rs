use serenity::all::Message;
use serenity::prelude::*;
use serenity::async_trait;

use crate::db::{db_conn, DbUser};
use crate::lastmessage::LastMessage;
use crate::counting::Counting;

/// This unit struct denominates all message handlers
/// via Serenity's EventHandler trait
pub struct MessageHandler;

#[async_trait]
impl EventHandler for MessageHandler {
    async fn message(&self, mut ctx: Context, msg: Message) {

        {
            let connection = &mut db_conn().unwrap();
            DbUser::new(connection, msg.author.id.into()).unwrap();
        }
        
        // Last Message
        LastMessage::handler(&mut ctx, &msg).await
            .unwrap();

        // Counting
        Counting::handler(&mut ctx, &msg).await
            .unwrap();
    }
}

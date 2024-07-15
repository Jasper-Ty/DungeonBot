use serenity::all::Message;
use serenity::prelude::*;
use serenity::async_trait;

use crate::lastmessage::LastMessage;

/// This unit struct denominates all message handlers
/// via Serenity's EventHandler trait
pub struct MessageHandler;

#[async_trait]
impl EventHandler for MessageHandler {
    async fn message(&self, mut ctx: Context, msg: Message) {
        
        // Last Message
        LastMessage::handler(&mut ctx, &msg).await
            .unwrap();
    }
}

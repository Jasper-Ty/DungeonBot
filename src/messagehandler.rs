use serenity::all::Message;
use serenity::prelude::*;
use serenity::async_trait;

use crate::lastmessage::lm_handler;

/// This unit struct denominates all message handlers
/// via Serenity's EventHandler trait
pub struct MessageHandler;

#[async_trait]
impl EventHandler for MessageHandler {
    async fn message(&self, mut ctx: Context, msg: Message) {
        
        // Last Message
        lm_handler(&mut ctx, &msg).await
            .unwrap();
    }
}

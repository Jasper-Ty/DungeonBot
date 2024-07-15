use std::sync::Arc;
use tokio::sync::RwLock; // Need async RwLock, will be holding across awaits
 
use serenity::prelude::*;
use serenity::all::Message;

use crate::error::{DungeonBotError, Result};

#[derive(Debug, Clone)]
pub struct CountingData {
    msg: Message,
    num: u64,
}

/// The underlying async data structure that holds the
/// last-message winner.
#[derive(Debug, Clone)]
pub struct CountingLock(Arc<RwLock<Option<CountingData>>>); 
impl CountingLock {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(None)))
    }
}

pub struct Counting;
impl TypeMapKey for Counting {
    type Value = CountingLock;
}
impl Counting {
    /// Get the LastMessage RwLock from Context
    pub async fn acquire_lock(ctx: &mut Context) -> Result<<Self as TypeMapKey>::Value> {
        ctx.data.read().await.get::<Self>()
            .ok_or(DungeonBotError::TypeMapKeyError("Counting".to_string()))
            .cloned()
    }
    pub async fn install(client: &mut Client) {
        let mut data = client.data.write().await;
        data.insert::<Self>(CountingLock::new());
    }

    pub async fn handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        Ok(())
    }
}

use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use serenity::all::Message;
use serenity::prelude::*;
use serenity::async_trait;

use thiserror::Error;

use crate::db::{db_conn, DbUser};
use crate::lastmessage::LastMessage;
use crate::counting::Counting;
use crate::error::{DungeonBotError, Result};

#[derive(Error, Debug)]
pub enum MsgSubsystemError {
    #[error("Error acquiring subsystem lock (read): {0}")]
    LockReadError(String),
    #[error("Error acquiring subsystem lock (write): {0}")]
    LockWriteError(String),
}

#[derive(Debug, Clone)]
struct MsgSubsystemLock<T: Clone + Send + Sync + Default>(Arc<RwLock<T>>);
impl<T> MsgSubsystemLock<T> 
where 
    T: Clone + Send + Sync + Default
{
    fn new() -> Self {
        return Self(Arc::new(RwLock::new(<T as Default>::default())));
    }

    pub fn read<'a>(&'a self) -> Result<RwLockReadGuard<T>>{
        self.0.read()
            .map_err(|e| MsgSubsystemError::LockReadError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }

    pub fn write(&self) -> Result<RwLockWriteGuard<T>>{
        self.0.write()
            .map_err(|e| MsgSubsystemError::LockWriteError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }
}

pub trait MsgSubsystem: TypeMapKey<Value=MsgSubsystemLock<Self::Data>> + Sized 
where 
    <Self as MsgSubsystem>::Data: Clone + Send + Sync + Default
{
    type Data;

    fn name() -> String;

    #[allow(async_fn_in_trait)]
    async fn lock(ctx: &mut Context) -> Result<Self::Value> {
        ctx.data.read().await.get::<Self>()
            .ok_or(DungeonBotError::TypeMapKeyError(Self::name()))
            .cloned()
    }

    #[allow(async_fn_in_trait)]
    async fn install_data(client: &mut Client) {
        let mut data = client.data.write().await;
        data.insert::<Self>(<Self as TypeMapKey>::Value::new());
    }

    #[allow(async_fn_in_trait)]
    async fn handler(ctx: &mut Context, msg: &Message);
}

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

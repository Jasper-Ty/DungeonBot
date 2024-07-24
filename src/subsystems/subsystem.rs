//! A subsystem of DungeonBot
//!

use serenity::prelude::*;

use crate::error::{DungeonBotError, Result};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SubsystemError {
    #[error("Error acquiring subsystem lock (read): {0}")]
    LockReadError(String),
    #[error("Error acquiring subsystem lock (write): {0}")]
    LockWriteError(String),
}

pub trait Data: Clone + Send + Sync + Sized + Default {}
impl<T: Clone + Send + Sync + Sized + Default> Data for T {}

pub trait SubsystemLock<T>: Data where T: Data {}

use std::{error::Error, sync::Arc};

#[derive(Debug, Clone, Default)]
pub struct SyncRwLock<T: Data>(Arc<std::sync::RwLock<T>>);

impl<T> SyncRwLock<T>
where 
    T: Data
{
    pub fn read(&self) -> Result<std::sync::RwLockReadGuard<T>> {
        self.0.read()
            .map_err(|e| SubsystemError::LockReadError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }

    pub fn write(&self) -> Result<std::sync::RwLockWriteGuard<T>> {
        self.0.write()
            .map_err(|e| SubsystemError::LockWriteError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }
}

impl<T> SubsystemLock<T> for SyncRwLock<T> where T: Data {}

#[derive(Debug, Clone, Default)]
pub struct AsyncRwLock<T: Clone + Send + Sync + Default>(Arc<tokio::sync::RwLock<T>>);

impl<T> AsyncRwLock<T> 
where 
    T: Data
{
    pub async fn read<'a>(&'a self) -> Result<tokio::sync::RwLockReadGuard<T>> {
        Ok(self.0.read().await)
    }

    pub async fn write<'a>(&'a self) -> Result<tokio::sync::RwLockWriteGuard<T>> {
        Ok(self.0.write().await)
    }
}

impl<T> SubsystemLock<T> for AsyncRwLock<T> where T: Data {}

use serenity::all::Message;

pub trait Subsystem: TypeMapKey + Sized
where 
    <Self as TypeMapKey>::Value: SubsystemLock<<Self as Subsystem>::Data>,
    <Self as Subsystem>::Data: Data
{
    type Data;

    #[allow(async_fn_in_trait)]
    async fn lock(ctx: &Context) -> Result<Self::Value> {
        ctx.data.read().await.get::<Self>()
            .ok_or(DungeonBotError::Unknown)
            .cloned()
    }

    fn data() -> <Self as TypeMapKey>::Value {
        <Self as TypeMapKey>::Value::default()
    }

    #[allow(async_fn_in_trait, unused_variables)]
    async fn message_handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        Ok(())
    }

    #[allow(async_fn_in_trait, unused_variables)]
    async fn reaction_handler(ctx: &mut Context, msg: &Message) -> Result<()> {
        Ok(())
    }

    #[allow(async_fn_in_trait)]
    async fn error_handler(ctx: &mut Context, msg: &Message, err: DungeonBotError) {
        let header = 
            "Oh noes, an error \
            <:flabbergasted:1250998996596555817>. \
            Please let Jasper know about this immediately.\n";

        let mut reply = header.to_string();
        if let Ok(channel_id) = msg.channel(&ctx.http).await {
            reply.push_str("```\n");
            reply.push_str("[Subsystem Error]\n");
            reply.push_str(&format!("{}\n", err));
            reply.push_str(&format!("{:?}\n", err));
            reply.push_str(&format!("{:?}\n", err.source()));
            reply.push_str("```");
            channel_id.id().say(&ctx.http, reply).await
                .expect("Unable to send error handler reply");
        }
    }
}

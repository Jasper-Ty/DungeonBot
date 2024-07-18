use std::fmt::Write;

use serenity::all::Message;
use serenity::prelude::*;

use crate::db::{db_conn, DbUser};
use crate::subsystems::{LastMessage, Counting};
use crate::error::{DungeonBotError, Result};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MsgSubsystemError {
    #[error("Error acquiring subsystem lock (read): {0}")]
    LockReadError(String),
    #[error("Error acquiring subsystem lock (write): {0}")]
    LockWriteError(String),
}

/// Tag trait that covers both std::sync::RwLock and tokio::sync::RwLock
pub trait RwLockType<T>: Clone + Send + Sync + Sized {
    fn new() -> Self;
}

use std::sync::Arc;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Clone)]
pub struct MsgSubsystemLock<T: Clone + Send + Sync + Default + Sized>(Arc<RwLock<T>>);
impl<T> MsgSubsystemLock<T> 
where 
    T: Clone + Send + Sync + Default + Sized
{
    pub fn read(&self) -> Result<RwLockReadGuard<T>> {
        self.0.read()
            .map_err(|e| MsgSubsystemError::LockReadError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }

    pub fn write(&self) -> Result<RwLockWriteGuard<T>> {
        self.0.write()
            .map_err(|e| MsgSubsystemError::LockWriteError(format!("{}", e)))
            .map_err(DungeonBotError::from)
    }
}

impl<T> RwLockType<T> for MsgSubsystemLock<T>
where 
    T: Clone + Send + Sync + Default + Sized
{
    fn new() -> Self {
        return Self(Arc::new(RwLock::new(<T as Default>::default())));
    }
}

use tokio::sync::RwLock as AsyncRwLock;
use tokio::sync::RwLockReadGuard as AsyncRwLockReadGuard;
use tokio::sync::RwLockWriteGuard as AsyncRwLockWriteGuard;

#[derive(Debug, Clone)]
pub struct MsgSubsystemAsyncLock<T: Clone + Send + Sync + Default>(Arc<AsyncRwLock<T>>);
impl<T> MsgSubsystemAsyncLock<T> 
where 
    T: Clone + Send + Sync + Default
{
    pub async fn read<'a>(&'a self) -> AsyncRwLockReadGuard<T> {
        self.0.read().await
    }

    pub async fn write<'a>(&'a self) -> AsyncRwLockWriteGuard<T> {
        self.0.write().await
    }
}

impl<T> RwLockType<T> for MsgSubsystemAsyncLock<T>
where 
    T: Clone + Send + Sync + Default + Sized
{
    fn new() -> Self {
        return Self(Arc::new(AsyncRwLock::new(<T as Default>::default())));
    }
}

pub trait MsgSubsystem: TypeMapKey + Sized 
where 
    <Self as TypeMapKey>::Value: RwLockType<<Self as MsgSubsystem>::Data>,
    <Self as MsgSubsystem>::Data: Clone + Send + Sync + Default
{
    type Data;

    fn name() -> String;

    #[allow(async_fn_in_trait)]
    async fn lock(ctx: &Context) -> Result<Self::Value> {
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
    async fn handler(ctx: &mut Context, msg: &Message) -> Result<()>;
}

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

        // Last Message
        match LastMessage::handler(&mut ctx, &msg).await {
            Err(err) => error_handler(&mut ctx, &msg, err).await,
            _ => {}
        }

        // Counting
        match Counting::handler(&mut ctx, &msg).await {
            Err(err) => error_handler(&mut ctx, &msg, err).await,
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

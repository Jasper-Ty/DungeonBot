use std::collections::HashMap;
use rand::prelude::*;

use crate::db::{db_conn, DbUser};
use crate::error::Result;
use super::{subsystem::SyncRwLock, Subsystem};

use serenity::prelude::*;
use serenity::all::{UserId, Message};

type TaxData = HashMap<UserId, i32>;
type TaxLock = SyncRwLock<TaxData>;

pub const TAX_RATE: f64 = 0.1;

pub struct Tax;
impl TypeMapKey for Tax {
    type Value = TaxLock;
}
impl Subsystem for Tax {
    type Data = TaxData;

    async fn message_handler(ctx: &mut Context, msg: &Message) -> Result<()> {

        let collect_tax = rand::thread_rng().gen::<f64>() > TAX_RATE;

        if collect_tax {

            let conn = &mut db_conn()?;
            DbUser::add_points(conn, msg.author.id.get(), -1)?;

            {
                let lock = Self::lock(ctx).await?;
                let mut write_lock = lock.write()?;

                let curr = write_lock.get(&msg.author.id)
                    .cloned()
                    .unwrap_or(0);

                write_lock.insert(msg.author.id, curr+1);
            }
        }

        Ok(())
    }
}

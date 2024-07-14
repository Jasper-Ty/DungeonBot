use diesel::backend::Backend;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::error::{DungeonBotError, Result};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations<DB: Backend>(conn: &mut impl MigrationHarness<DB>) -> Result<()> {

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DungeonBotError::MigrationError(e))?;

    Ok(())
}

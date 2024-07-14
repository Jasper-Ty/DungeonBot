pub mod models;
pub mod schema;

mod migrations;

pub use migrations::run_migrations;

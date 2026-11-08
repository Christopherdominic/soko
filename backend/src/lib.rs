pub mod config;
pub mod errors;
pub mod handlers;
pub mod lightning;
pub mod routes;
pub mod state;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

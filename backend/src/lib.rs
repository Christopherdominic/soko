pub mod config;
pub mod errors;
pub mod handlers;
pub mod lightning;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

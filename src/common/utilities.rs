use dotenv::dotenv;
use log::{error, info, trace};
use log4rs;
use serde_yaml;
use sqlx::{database, PgPool};
use std::env;

// ERROR
// WARN
// INFO
// DEBUG
// TRACE

// info!("Goes to console, file and rolling file");
// error!("Goes to console, file and rolling file");
// trace!("Doesn't go to console as it is filtered out");

pub fn log_init() {
    let config_str = include_str!("log_config.yml");
    let config = serde_yaml::from_str(config_str).unwrap();
    log4rs::init_raw_config(config).unwrap();
}

pub async fn db_init() -> Result<(PgPool), sqlx::Error> {
    // dotenv().ok();

    // let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database_url = "postgres://postgres:password@localhost/cluster".to_string();

    let pool = PgPool::connect(&database_url).await.expect("");
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("Connected to the database ✅");

    Ok(pool)
}
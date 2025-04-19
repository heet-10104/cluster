use log::info;
use log4rs;
use serde_yaml;
use sqlx::PgPool;
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
    let config = serde_yaml::from_str(config_str).expect("error parsing log config");

    log4rs::init_raw_config(config).expect("logger failed to initialize");
}

pub async fn db_init() -> Result<PgPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("connected to database");
    Ok(pool)
}

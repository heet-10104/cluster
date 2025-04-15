use log::{error, info, trace};
use log4rs;
use serde_yaml;

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

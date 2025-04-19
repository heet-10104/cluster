use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;
use std::{thread, time::Duration};

use serde_json::Error as SerdeError;
use std::fs;
use std::io::Error as IoError;

use crate::config::loadbalancer_config::LoadBalancerConfig;
use crate::config::server_config::ServerConfig;
use crate::subapps::loadbalancer::ApiConfig;

pub fn validate_lb_config(config: &LoadBalancerConfig) -> bool {
    let ips: Vec<String> = config.nodes.iter().map(|ip| ip.clone()).collect();
    let mut is_valid = true;

    let bar = ProgressBar::new(ips.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} Checking {msg}")
            .unwrap(),
    );
    for ip in &ips {
        bar.set_message(ip.clone());
        if is_ip_live(ip) {
            println!("{} is live", ip);
        } else {
            is_valid = false;
            println!("{} is not reachable", ip);
        }
        bar.inc(1);
        thread::sleep(Duration::from_millis(500));
    }
    bar.finish_with_message("Validation complete.");
    is_valid
}

pub fn is_ip_live(ip: &str) -> bool {
    let output = Command::new("ping").arg("-c").arg("1").arg(ip).output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

pub fn validate_server_config(config: &ServerConfig) -> bool {
    let ip = &config.ip;
    let mut valid = true;
    if is_ip_live(&ip) {
        println!("server {} is live", ip);
    } else {
        println!("server {} is not reachable", ip);
        let _ = !valid;
    }

    let loadblancers: Vec<String> = config.loadbalancer_ip.iter().map(|ip| ip.clone()).collect();
    let bar = ProgressBar::new(loadblancers.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} Checking {msg}")
            .unwrap(),
    );
    for ip in &loadblancers {
        bar.set_message(ip.clone());
        if is_ip_live(ip) {
            println!("{} is live", ip);
        } else {
            valid = false;
            println!("{} is not reachable", ip);
        }
        bar.inc(1);
        thread::sleep(Duration::from_millis(500));
    }
    bar.finish_with_message("Validation complete.");
    valid
}

pub fn read_json_from_file(file_path: &str) -> Result<String, IoError> {
    fs::read_to_string(file_path)
}

pub fn validate_person_json(json_str: &str) -> Result<ApiConfig, SerdeError> {
    serde_json::from_str::<ApiConfig>(json_str)
}

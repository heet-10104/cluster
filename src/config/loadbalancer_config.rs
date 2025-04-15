use confy;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;
use std::{thread, time::Duration};
use tokio::runtime::Runtime;

use crate::subapps::loadbalancer::balance_load;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Features {
    HealthCheck,
    ApiHealthCheck,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Protocol {
    RobinRound,
    LeastConnections,
    LeastResponse,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::RobinRound
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct LoadBalancerConfig {
    pub ip: String,
    pub protocol: Protocol,
    pub features: Vec<Features>,
    pub nodes: Vec<String>,
}

impl ToString for Features {
    fn to_string(&self) -> String {
        let string = match self {
            Features::HealthCheck => "Health Check",
            Features::ApiHealthCheck => "Api Health Check",
        };
        string.into()
    }
}

impl ToString for Protocol {
    fn to_string(&self) -> String {
        let string = match self {
            Protocol::RobinRound => "Robin Round",
            Protocol::LeastConnections => "Least Connections",
            Protocol::LeastResponse => "Leas tResponse",
        };
        string.into()
    }
}

pub fn configure_load_balancer(protocols: &[Protocol], features: &[Features]) {
    let protocol = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Protocol")
        .default(0)
        .items(&protocols)
        .interact()
        .unwrap();
    let protocol = &protocols[protocol];

    let defaults = &[true, false];
    let features_selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select Features")
        .items(&features)
        .defaults(&defaults[..])
        .interact()
        .unwrap();
    let selected_features: Vec<Features> = features_selected
        .iter()
        .map(|&index| features[index].clone())
        .collect();

    let ip: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Load Balancer IPv4")
        .validate_with(|input: &String| -> Result<(), &str> {
            let numbers: Vec<&str> = input.split('.').collect();

            if numbers.len() != 4 {
                return Err("IPv4 address must have exactly 4 octets");
            }

            for num in numbers {
                match num.parse::<u8>() {
                    Ok(_) => continue,
                    Err(_) => return Err("Each octet must be a number between 0 and 255"),
                }
            }

            Ok(())
        })
        .interact_text()
        .unwrap();

    let mut next_ip = true;
    let mut nodes: Vec<String> = vec![];
    while next_ip {
        let node_ip: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Connected Server IPv4")
            .validate_with(|input: &String| -> Result<(), &str> {
                let numbers: Vec<&str> = input.split('.').collect();

                if numbers.len() != 4 {
                    return Err("IPv4 address must have exactly 4 octets");
                }

                for num in numbers {
                    match num.parse::<u8>() {
                        Ok(_) => continue,
                        Err(_) => return Err("Each octet must be a number between 0 and 255"),
                    }
                }

                Ok(())
            })
            .interact_text()
            .unwrap();
        nodes.push(node_ip);

        let add_more = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Add more nodes?")
            .default(false)
            .interact()
            .unwrap();
        if !add_more {
            next_ip = false;
        }
    }
    let config = LoadBalancerConfig {
        ip: ip.clone(),
        protocol: protocol.clone(),
        features: selected_features.clone(),
        nodes: nodes.clone(),
    };
    println!("Load Balancer Config: {:#?}", config);

    if !validate_config(&config) {
        println!("Invalid configuration. Please check the IP addresses and try again.");
        return;
    }
    confy::store("load-balancer-config", None, config).expect("Failed to store config");
    let rt = Runtime::new().unwrap();
    rt.block_on(balance_load());
}

fn validate_config(config: &LoadBalancerConfig) -> bool {
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
fn is_ip_live(ip: &str) -> bool {
    let output = Command::new("ping").arg("-c").arg("1").arg(ip).output();

    match output {
        Ok(result) => result.status.success(),
        Err(_) => false,
    }
}

use crate::validator::validate::validate_lb_config;
use confy::{self, ConfyError};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, MultiSelect, Select};
use log::{error, info};

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

pub async fn configure_load_balancer(
    protocols: &[Protocol],
    features: &[Features],
) -> Result<(), ConfyError> {
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
    info!("Load Balancer Config: {:#?}", config);

    if !validate_lb_config(&config) {
        error!("Invalid configuration. Please check the IP addresses and try again.");
        std::process::exit(1);
    }
    confy::store("load-balancer-config", None, config)
}
